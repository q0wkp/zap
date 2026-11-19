use std::{
    collections::{HashMap, HashSet},
    future::Future,
    path::{Path, PathBuf},
    sync::mpsc::{self, channel},
    thread,
    time::Duration,
};

pub mod home_watcher;
pub use home_watcher::{HomeDirectoryWatcher, HomeDirectoryWatcherEvent};

use anyhow::Result;
use futures::channel::oneshot;
use notify_debouncer_full::{
    new_debouncer_opt,
    notify::{
        self,
        event::{ModifyKind, RenameMode},
        EventKind, RecommendedWatcher, RecursiveMode, WatchFilter,
    },
    DebounceEventHandler, DebounceEventResult, DebouncedEvent, Debouncer, NoCache,
};
use warpui::{Entity, ModelContext};

#[derive(Debug)]
enum BackgroundFileWatcherCommand {
    AddPath {
        path: PathBuf,
        filter: WatchFilter,
        response: oneshot::Sender<Result<()>>,
        recursive_mode: RecursiveMode,
    },
    RemovePath {
        path: PathBuf,
        response: oneshot::Sender<Result<()>>,
    },
}

struct BackgroundFileWatcher {
    notifier: Debouncer<RecommendedWatcher, NoCache>,
    rx: mpsc::Receiver<BackgroundFileWatcherCommand>,
}

impl BackgroundFileWatcher {
    fn new(
        debounce_duration: Duration,
        handler: WatcherEventHandler,
        rx: mpsc::Receiver<BackgroundFileWatcherCommand>,
    ) -> Self {
        let debounced_watcher = new_debouncer_opt(
            debounce_duration,
            None,
            handler,
            NoCache,
            notify::Config::default(),
        )
        .expect("Should be able to create watcher");

        Self {
            notifier: debounced_watcher,
            rx,
        }
    }

    /// Listen to streamed in commands to modify the internal notifier state.
    fn run(mut self) {
        while let Ok(res) = self.rx.recv() {
            match res {
                BackgroundFileWatcherCommand::AddPath {
                    path,
                    filter,
                    response,
                    recursive_mode,
                } => {
                    let _ = response.send(
                        self.notifier
                            .watch_filtered(path, recursive_mode, filter)
                            .inspect_err(|err| {
                                log::warn!("Failed to watch path: {err:?}");
                            })
                            .map_err(anyhow::Error::new),
                    );
                }
                BackgroundFileWatcherCommand::RemovePath { path, response } => {
                    let _ = response.send(
                        self.notifier
                            .unwatch(path)
                            .inspect_err(|err| {
                                log::warn!("Failed to remove repo watcher: {err:?}");
                            })
                            .map_err(anyhow::Error::new),
                    );
                }
            }
        }
        log::debug!("File watcher stream closed")
    }
}

#[derive(Clone, Default, Debug)]
pub struct BulkFilesystemWatcherEvent {
    /// Paths that were created.
    pub added: HashSet<PathBuf>,

    /// Paths whose contents were modified.
    pub modified: HashSet<PathBuf>,

    /// List of paths that should be removed.
    pub deleted: HashSet<PathBuf>,

    /// Mapping from rename target to rename source.
    pub moved: HashMap<PathBuf, PathBuf>,
}

impl BulkFilesystemWatcherEvent {
    /// Iterator over paths that were added or modified.
    pub fn added_or_updated_iter(&self) -> impl Iterator<Item = &PathBuf> {
        self.added.iter().chain(self.modified.iter())
    }

    /// Returns an owned set of paths that were added or modified.
    ///
    /// Prefer `added_or_updated_iter` when you don't need ownership.
    pub fn added_or_updated_set(&self) -> HashSet<PathBuf> {
        self.added_or_updated_iter().cloned().collect()
    }

    fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.modified.is_empty()
            && self.deleted.is_empty()
            && self.moved.is_empty()
    }
}

/// Validates a path before it reaches the platform watcher.
///
/// An empty path is not a location, and the platform backends handle it badly enough to be fatal:
/// on macOS `notify` cannot build a CoreFoundation URL for it and then releases a null `CFError`,
/// which traps and takes the whole process down. Rejecting it here keeps a caller's bad path from
/// being a crash. See Sentry issue
/// [WARP-CLIENT-DEV-XT3](https://warpdotdev.sentry.io/issues/WARP-CLIENT-DEV-XT3).
fn ensure_watchable_path(path: &Path) -> Result<PathBuf> {
    if path.as_os_str().is_empty() {
        anyhow::bail!("the filesystem watcher cannot watch an empty path");
    }
    Ok(path.to_path_buf())
}

/// Model for watching for all file / folder changes under target directories.
/// The updates are debounced with a configurable duration.
pub struct BulkFilesystemWatcher {
    tx: mpsc::Sender<BackgroundFileWatcherCommand>,
}

impl BulkFilesystemWatcher {
    pub fn new(debounce_duration: Duration, ctx: &mut ModelContext<Self>) -> Self {
        let (tx, rx) = async_channel::unbounded();
        let (bg_tx, bg_rx) = channel();

        // Note that we keep the file watcher in the background since registering and unregistering file path
        // involves fs calls.
        if let Err(e) = thread::Builder::new()
            .name("Bulk Filesystem Watcher".into())
            .spawn(move || {
                let watcher = BackgroundFileWatcher::new(
                    debounce_duration,
                    WatcherEventHandler { tx },
                    bg_rx,
                );
                watcher.run();
            })
        {
            log::error!("Failed to spawn thread for background file watcher {e:?}");
        }
        ctx.spawn_stream_local(rx, Self::handle_watcher_event, |_, _| {});

        Self { tx: bg_tx }
    }

    pub fn new_for_test() -> Self {
        let (bg_tx, _) = channel();
        Self { tx: bg_tx }
    }

    /// Stop watching a path. The returned future resolves once the path is fully unregistered.
    /// Awaiting the future is *not* required for the path to be unregistered.
    pub fn unregister_path(&mut self, path: &Path) -> impl Future<Output = Result<()>> {
        let (tx, rx) = oneshot::channel();
        let send_result = ensure_watchable_path(path).and_then(|path| {
            self.tx
                .send(BackgroundFileWatcherCommand::RemovePath { path, response: tx })
                .map_err(anyhow::Error::new)
        });

        if let Err(error) = &send_result {
            log::warn!("Failed to unregister watched path: {error:#}");
        }

        async move {
            send_result?;
            rx.await.map_err(anyhow::Error::new)??;
            Ok(())
        }
    }

    /// Add a new path to watch. The returned future resolves once the path is fully registered.
    /// Awaiting the future is *not* required for the path to be registered.
    pub fn register_path(
        &mut self,
        path: &Path,
        watch_filter: WatchFilter,
        recursive_mode: RecursiveMode,
    ) -> impl Future<Output = Result<()>> {
        let (tx, rx) = oneshot::channel();
        let send_result = ensure_watchable_path(path).and_then(|path| {
            self.tx
                .send(BackgroundFileWatcherCommand::AddPath {
                    path,
                    filter: watch_filter,
                    response: tx,
                    recursive_mode,
                })
                .map_err(anyhow::Error::new)
        });

        if let Err(error) = &send_result {
            log::warn!("Failed to register watched path: {error:#}");
        }

        async move {
            send_result?;
            rx.await.map_err(anyhow::Error::new)??;
            Ok(())
        }
    }

    fn handle_watcher_event(
        &mut self,
        event: BulkFilesystemWatcherEvent,
        ctx: &mut ModelContext<Self>,
    ) {
        ctx.emit(event);
    }
}

impl Entity for BulkFilesystemWatcher {
    type Event = BulkFilesystemWatcherEvent;
}

struct WatcherEventHandler {
    tx: async_channel::Sender<BulkFilesystemWatcherEvent>,
}

impl DebounceEventHandler for WatcherEventHandler {
    fn handle_event(&mut self, result: DebounceEventResult) {
        match result {
            Ok(debounce_events) => {
                if let Ok(config_event) =
                    deduplicate_and_merge_raw_notifier_events(&debounce_events)
                {
                    if let Err(e) = self.tx.try_send(config_event) {
                        log::warn!("Failed to send WatcherEvent: {e:?}");
                    }
                }
            }
            Err(e) => {
                log::warn!("Received error in repo watcher: {e:?}");
            }
        }
    }
}

/// Records that `path` no longer exists.
///
/// If a path is created and then removed, it is dropped from the update. If a path is modified or moved
/// and then removed, only the removal is kept, and for a moved path it is the source that is removed
/// instead of the target name.
fn record_removed_path(
    update: &mut BulkFilesystemWatcherEvent,
    created: &mut HashSet<PathBuf>,
    modified: &mut HashSet<PathBuf>,
    path: &Path,
) {
    if created.remove(path) {
        return;
    }

    modified.remove(path);

    update.deleted.insert(match update.moved.remove(path) {
        Some(source) => source,
        None => path.to_path_buf(),
    });
}

/// Records a rename, squashing it with an earlier rename of the same path in this update.
fn record_moved_path(update: &mut BulkFilesystemWatcherEvent, from: PathBuf, to: PathBuf) {
    match update.moved.remove(&from) {
        Some(source) => update.moved.insert(to, source),
        None => update.moved.insert(to, from),
    };
}

/// Dedupe and standardize the raw notifier events into a BulkFilesystemWatcherEvent.
fn deduplicate_and_merge_raw_notifier_events(
    raw_fs_events: &[DebouncedEvent],
) -> Result<BulkFilesystemWatcherEvent> {
    let mut update = BulkFilesystemWatcherEvent::default();

    let mut created: HashSet<PathBuf> = HashSet::new();
    let mut modified: HashSet<PathBuf> = HashSet::new();

    // The source of a rename whose target has not been seen yet, with the tracker the platform gave it.
    let mut rename_from: Option<(PathBuf, Option<usize>)> = None;
    for fs_event in raw_fs_events {
        match fs_event.event.kind {
            // Create and modify should always be preserved.
            EventKind::Create(_) => created.extend(fs_event.event.paths.clone()),
            // On Windows, ReadDirectoryChangesW emits ModifyKind::Any instead of
            // ModifyKind::Data for file content changes. Handle both variants.
            EventKind::Modify(ModifyKind::Data(_) | ModifyKind::Any) => {
                modified.extend(fs_event.event.paths.clone())
            }

            // If a path is created and then removed, we should not keep this path in the update event.
            // If a path is modified / moved and then removed, we should only keep the remove event.
            EventKind::Remove(_) => {
                for path in &fs_event.event.paths {
                    record_removed_path(&mut update, &mut created, &mut modified, path);
                }
            }

            // Note that in MacOS, deleting could be either 1) a true removal 2) moving to trash. In the
            // second case, this event will come in as a EventKind::Modify on the name with RenameMode::Any.
            // Here we count this as a OutlineUpdate::remove.
            //
            // Another case of EventKind::Modify(ModifyKind::Name(RenameMode::Any)) is when a file was renamed
            // when we turn off file map caching. Since we cannot guarantee mapping from the old name to the new name
            // (this is ordering is unfortunately not strictly persisted in the event. e.g. rename A -> B and B -> A might
            // create an event stream of rename A, rename A, rename B, rename B), we will treating them as inserts and removes
            // for now based on the current state of the file system.
            EventKind::Modify(ModifyKind::Name(RenameMode::Any)) => {
                let is_rename = matches!(
                    fs_event.event.kind,
                    EventKind::Modify(ModifyKind::Name(RenameMode::Any))
                );
                for path in &fs_event.event.paths {
                    // Decides whether this is a rename to or rename from based on the current state of the file system.
                    // This is not ideal since when we receive the event, the state of the file system could have changed.
                    // E.g. rename A -> B, B -> C, if we receives the first event after B is already renamed to C, this will
                    // translate the events to delete (A, B), create (C).
                    // TODO(kevin)
                    let path_exists = is_rename && path.exists();

                    if path_exists {
                        created.insert(path.clone());
                        continue;
                    }

                    record_removed_path(&mut update, &mut created, &mut modified, path);
                }
            }

            // A rename the debouncer could correlate arrives as `Both`. Separate `From` and `To` events are
            // only two halves of one rename when the platform reports no tracker for either (Windows) or the
            // same tracker for both. Any other `From` is a path that left the watched tree, and any other `To`
            // is a path that entered it.
            EventKind::Modify(ModifyKind::Name(rename_mode)) => {
                let paths = &fs_event.event.paths;
                let tracker = fs_event.event.tracker();

                match rename_mode {
                    RenameMode::From => {
                        if let Some((unmatched, _)) = rename_from.take() {
                            record_removed_path(
                                &mut update,
                                &mut created,
                                &mut modified,
                                &unmatched,
                            );
                        }
                        if let Some(from) = paths.first() {
                            rename_from = Some((from.clone(), tracker));
                        }
                    }
                    RenameMode::To => {
                        let Some(to) = paths.first() else {
                            continue;
                        };
                        match rename_from.take() {
                            Some((from, from_tracker)) if from_tracker == tracker => {
                                record_moved_path(&mut update, from, to.clone());
                            }
                            Some((unmatched, _)) => {
                                record_removed_path(
                                    &mut update,
                                    &mut created,
                                    &mut modified,
                                    &unmatched,
                                );
                                created.insert(to.clone());
                            }
                            None => {
                                created.insert(to.clone());
                            }
                        }
                    }
                    RenameMode::Both => {
                        if let [from, to, ..] = paths.as_slice() {
                            record_moved_path(&mut update, from.clone(), to.clone());
                        }
                    }
                    RenameMode::Any | RenameMode::Other => {}
                }
            }
            _ => (),
        }
    }

    // A rename source that never got its target was moved out of the watched tree.
    if let Some((unmatched, _)) = rename_from {
        record_removed_path(&mut update, &mut created, &mut modified, &unmatched);
    }

    // A path that is created and then modified within the debounce window should be considered "added".
    for path in &created {
        modified.remove(path);
    }

    update.added = created;
    update.modified = modified;

    if update.is_empty() {
        return Err(anyhow::anyhow!("No update event produced"));
    }

    Ok(update)
}

#[cfg(test)]
#[path = "lib_test.rs"]
mod tests;

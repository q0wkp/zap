// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: added tests that paths moved into or out of the watched tree are reported.

use std::time::Instant;

use notify_debouncer_full::notify::Event;

use super::*;

/// APP-5243 / WARP-CLIENT-DEV-XT3: an empty path must never reach the platform watcher, because
/// macOS turns it into a null-`CFError` release that traps and kills the process.
#[test]
fn empty_paths_never_reach_the_platform_watcher() {
    assert!(ensure_watchable_path(Path::new("")).is_err());

    let directory = std::env::temp_dir();
    assert_eq!(ensure_watchable_path(&directory).ok(), Some(directory));
}

fn rename_event(mode: RenameMode, paths: &[&str], tracker: Option<usize>) -> DebouncedEvent {
    let mut event = Event::new(EventKind::Modify(ModifyKind::Name(mode)));
    for path in paths {
        event = event.add_path(PathBuf::from(path));
    }
    if let Some(tracker) = tracker {
        event = event.set_tracker(tracker);
    }
    DebouncedEvent::new(event, Instant::now())
}

fn paths(paths: &[&str]) -> HashSet<PathBuf> {
    paths.iter().map(PathBuf::from).collect()
}

/// A `From` without a target is a path that was moved out of the watched tree.
#[test]
fn rename_from_without_target_is_deleted() {
    let update = deduplicate_and_merge_raw_notifier_events(&[rename_event(
        RenameMode::From,
        &["/watched/out"],
        Some(1),
    )])
    .expect("a path leaving the watched tree is an update");

    assert_eq!(update.deleted, paths(&["/watched/out"]));
    assert!(update.added.is_empty());
    assert!(update.moved.is_empty());
}

/// A `To` without a source is a path that was moved into the watched tree.
#[test]
fn rename_to_without_source_is_added() {
    let update = deduplicate_and_merge_raw_notifier_events(&[rename_event(
        RenameMode::To,
        &["/watched/in"],
        Some(1),
    )])
    .expect("a path entering the watched tree is an update");

    assert_eq!(update.added, paths(&["/watched/in"]));
    assert!(update.deleted.is_empty());
    assert!(update.moved.is_empty());
}

/// One path leaving the tree and another entering it are not a rename, even when the two events are
/// adjacent: their trackers differ.
#[test]
fn unrelated_rename_from_and_to_are_not_paired() {
    let update = deduplicate_and_merge_raw_notifier_events(&[
        rename_event(RenameMode::From, &["/watched/first-out"], Some(1)),
        rename_event(RenameMode::From, &["/watched/second-out"], Some(2)),
        rename_event(RenameMode::To, &["/watched/in"], Some(3)),
    ])
    .expect("the three renames are an update");

    assert_eq!(
        update.deleted,
        paths(&["/watched/first-out", "/watched/second-out"])
    );
    assert_eq!(update.added, paths(&["/watched/in"]));
    assert!(update.moved.is_empty());
}

/// Platforms that report no tracker (Windows) still get a `From` followed by a `To` as one rename.
#[test]
fn untracked_rename_from_and_to_are_a_move() {
    let update = deduplicate_and_merge_raw_notifier_events(&[
        rename_event(RenameMode::From, &["/watched/from"], None),
        rename_event(RenameMode::To, &["/watched/to"], None),
    ])
    .expect("a rename is an update");

    assert_eq!(
        update.moved,
        HashMap::from([(PathBuf::from("/watched/to"), PathBuf::from("/watched/from"))])
    );
    assert!(update.added.is_empty());
    assert!(update.deleted.is_empty());
}

#[test]
fn correlated_rename_is_a_move() {
    let update = deduplicate_and_merge_raw_notifier_events(&[rename_event(
        RenameMode::Both,
        &["/watched/from", "/watched/to"],
        Some(1),
    )])
    .expect("a rename is an update");

    assert_eq!(
        update.moved,
        HashMap::from([(PathBuf::from("/watched/to"), PathBuf::from("/watched/from"))])
    );
    assert!(update.added.is_empty());
    assert!(update.deleted.is_empty());
}

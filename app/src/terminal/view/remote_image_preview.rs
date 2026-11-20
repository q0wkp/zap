//! Image previews of link tooltips for files on a remote host.
//!
//! The remote server sends a thumbnail instead of the file, and each terminal view keeps the
//! thumbnails it fetched: opening a tooltip again shows the image at once, while the server
//! is only asked whether the file changed. A thumbnail is dropped a day after it was last
//! shown, and all of them go with the view.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use remote_server::proto::{read_image_thumbnail_response, ImageFileVersion};
use zap_ng_core::HostId;
use zap_ng_ui::{
    assets::asset_cache::{AssetCache, AssetSource},
    image_cache::ImageType,
    r#async::Timer,
    SingletonEntity, ViewContext,
};

use super::{
    tooltips::{
        image_preview_size, LinkImagePreview, IMAGE_PREVIEW_MAX_HEIGHT, IMAGE_PREVIEW_MAX_WIDTH,
    },
    TerminalView,
};
use crate::remote_server::manager::RemoteServerManager;

/// How long a thumbnail is kept after it was last shown.
const THUMBNAIL_TIME_TO_LIVE: Duration = Duration::from_secs(24 * 60 * 60);

/// A thumbnail has this many pixels for each point the preview is drawn at, so that it stays
/// sharp on a high-density display.
const THUMBNAIL_PIXELS_PER_POINT: u32 = 2;

struct CachedThumbnail {
    /// The version of the remote file the thumbnail was made from.
    version: ImageFileVersion,
    /// The encoded thumbnail.
    bytes: Arc<[u8]>,
    last_shown: Instant,
}

/// The thumbnails a terminal view fetched from remote hosts, by host and path.
#[derive(Default)]
pub(super) struct RemoteImagePreviewCache {
    thumbnails: HashMap<(HostId, PathBuf), CachedThumbnail>,
    /// Numbers the assets the view puts into the asset cache, so that no two share an id.
    next_asset_number: u64,
}

impl RemoteImagePreviewCache {
    fn remove_expired(&mut self) {
        self.thumbnails
            .retain(|_, thumbnail| thumbnail.last_shown.elapsed() < THUMBNAIL_TIME_TO_LIVE);
    }
}

impl TerminalView {
    /// Shows the preview of the image at `path` on the remote host: the cached thumbnail at
    /// once, if there is one, and the current one when the server has answered, if the
    /// tooltip is still open for that file then.
    pub(super) fn show_remote_link_image_preview(
        &mut self,
        host_id: HostId,
        path: PathBuf,
        ctx: &mut ViewContext<Self>,
    ) {
        self.remote_image_previews.remove_expired();

        let key = (host_id, path);
        let cached = self
            .remote_image_previews
            .thumbnails
            .get_mut(&key)
            .map(|thumbnail| {
                thumbnail.last_shown = Instant::now();
                (thumbnail.version.clone(), thumbnail.bytes.clone())
            });
        let (known_version, image) = match cached {
            Some((version, bytes)) => {
                self.schedule_remote_thumbnail_expiry(ctx);
                (Some(version), Some(self.thumbnail_image(&bytes, ctx)))
            }
            None => (None, None),
        };
        self.set_link_image_preview(
            Some(LinkImagePreview {
                path: key.1.clone(),
                image,
            }),
            ctx,
        );

        let Some(path_str) = key.1.to_str().map(str::to_owned) else {
            return;
        };
        let Some(client) = RemoteServerManager::handle(ctx)
            .as_ref(ctx)
            .client_for_host(&key.0)
            .cloned()
        else {
            return;
        };
        ctx.spawn(
            async move {
                client
                    .read_image_thumbnail(
                        path_str,
                        IMAGE_PREVIEW_MAX_WIDTH as u32 * THUMBNAIL_PIXELS_PER_POINT,
                        IMAGE_PREVIEW_MAX_HEIGHT as u32 * THUMBNAIL_PIXELS_PER_POINT,
                        known_version,
                    )
                    .await
                    .map_err(|error| format!("{error}"))
                    .and_then(|response| match response.result {
                        Some(read_image_thumbnail_response::Result::Success(success)) => {
                            Ok(success)
                        }
                        Some(read_image_thumbnail_response::Result::Error(error)) => {
                            Err(error.message)
                        }
                        None => Err("the response has no result".to_owned()),
                    })
            },
            move |me, result, ctx| {
                let success = match result {
                    Ok(success) => success,
                    Err(error) => {
                        log::warn!(
                            "Failed to load the preview of remote image {}: {error}",
                            key.1.display()
                        );
                        return;
                    }
                };
                // No image: the file is still at the version of the thumbnail on show.
                let (Some(version), Some(bytes)) = (success.version, success.image) else {
                    return;
                };
                let bytes: Arc<[u8]> = bytes.into();
                me.remote_image_previews.thumbnails.insert(
                    key.clone(),
                    CachedThumbnail {
                        version,
                        bytes: bytes.clone(),
                        last_shown: Instant::now(),
                    },
                );
                me.schedule_remote_thumbnail_expiry(ctx);

                // The tooltip may have closed, or opened for another link, meanwhile.
                if me.is_link_image_preview_open_for(&key.1) {
                    let image = me.thumbnail_image(&bytes, ctx);
                    me.set_link_image_preview(
                        Some(LinkImagePreview {
                            path: key.1,
                            image: Some(image),
                        }),
                        ctx,
                    );
                    ctx.notify();
                }
            },
        );
    }

    fn is_link_image_preview_open_for(&self, path: &Path) -> bool {
        self.link_image_preview
            .as_ref()
            .is_some_and(|preview| preview.path == path)
    }

    /// Puts a thumbnail into the asset cache, where it stays only while its preview is on
    /// show, and returns it with the size to draw it at.
    fn thumbnail_image(
        &mut self,
        bytes: &[u8],
        ctx: &mut ViewContext<Self>,
    ) -> (AssetSource, f32, f32) {
        let dimensions = image::ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .ok()
            .and_then(|reader| reader.into_dimensions().ok());
        let (width, height) = image_preview_size(dimensions);

        let asset_number = self.remote_image_previews.next_asset_number;
        self.remote_image_previews.next_asset_number += 1;
        let asset_id = format!("link-image-preview:{}:{asset_number}", ctx.view_id());
        AssetCache::handle(ctx).update(ctx, |cache, ctx| {
            cache.insert_raw_asset_bytes::<ImageType>(asset_id.clone(), bytes, ctx);
        });
        (AssetSource::Raw { id: asset_id }, width, height)
    }

    /// Drops the thumbnails that are past their time to live once the one just shown would
    /// be, if it is not shown again.
    fn schedule_remote_thumbnail_expiry(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.spawn(
            Timer::after(THUMBNAIL_TIME_TO_LIVE + Duration::from_secs(1)),
            |me, _, _| me.remote_image_previews.remove_expired(),
        );
    }
}

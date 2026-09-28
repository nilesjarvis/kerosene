use crate::app_time::now_ms;
use crate::telegram_feed::{
    TelegramChannelProfile, TelegramFastFeedEvent, TelegramFeedPage, TelegramFeedPost,
    TelegramMediaKind, is_supported_raster_image,
};
use futures::{SinkExt as _, channel::mpsc};
use grammers_client::Client;
use grammers_client::media::{Document, Downloadable, Media};
use grammers_client::peer::Peer;
use iced::widget::image::Handle as ImageHandle;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

const TELEGRAM_PRIVATE_CANDIDATE_AVATAR_MAX_BYTES: usize = 128 * 1024;
const TELEGRAM_FAST_MEDIA_MAX_BYTES: usize = 2 * 1024 * 1024;
const TELEGRAM_FAST_MEDIA_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(20);

pub(super) async fn download_private_channel_avatar_handle(
    client: &Client,
    peer: Peer,
) -> Option<ImageHandle> {
    let photo = peer.photo(false).await?;
    let bytes =
        download_downloadable_bytes(client, &photo, TELEGRAM_PRIVATE_CANDIDATE_AVATAR_MAX_BYTES)
            .await?;
    Some(ImageHandle::from_bytes(bytes))
}

async fn download_downloadable_bytes<D: Downloadable>(
    client: &Client,
    downloadable: &D,
    max_bytes: usize,
) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut download = client.iter_download(downloadable);
    while let Some(chunk) = download.next().await.ok()? {
        bytes.extend_from_slice(&chunk);
        if bytes.len() > max_bytes {
            return None;
        }
    }
    (!bytes.is_empty()).then_some(bytes)
}

/// Classifies a message's media into a renderable preview kind, or `None` for
/// media we never show inline (polls, contacts, generic files, animated
/// stickers, …). Kept in sync with [`download_fast_post_media`].
pub(super) fn fast_media_kind(media: &Media) -> Option<TelegramMediaKind> {
    match media {
        Media::Photo(_) => Some(TelegramMediaKind::Photo),
        Media::Sticker(sticker) => (!sticker.is_animated()).then_some(TelegramMediaKind::Sticker),
        Media::Document(document) => fast_document_media_kind(document),
        _ => None,
    }
}

fn fast_document_media_kind(document: &Document) -> Option<TelegramMediaKind> {
    let mime = document
        .mime_type()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let is_gif = document.is_animated() || mime == "image/gif";
    if mime.starts_with("video/") || is_gif {
        Some(if is_gif {
            TelegramMediaKind::Gif
        } else {
            TelegramMediaKind::Video
        })
    } else if mime.starts_with("image/") {
        Some(TelegramMediaKind::Photo)
    } else {
        None
    }
}

/// Downloads a renderable preview image for a message's media, in memory only.
/// Photos and static stickers download in full (already compressed and small);
/// videos and GIFs would be far too large to inline, so only their
/// server-rendered preview thumbnail is fetched.
async fn download_fast_post_media(
    client: &Client,
    message: &grammers_client::message::Message,
) -> Option<ImageHandle> {
    let media = message.media()?;
    let bytes = match media {
        Media::Photo(photo) => {
            download_downloadable_bytes(client, &photo, TELEGRAM_FAST_MEDIA_MAX_BYTES).await?
        }
        Media::Sticker(sticker) => {
            if sticker.is_animated() {
                return None;
            }
            let document = &sticker.document;
            let mime = document
                .mime_type()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if mime == "image/webp" {
                download_downloadable_bytes(client, document, TELEGRAM_FAST_MEDIA_MAX_BYTES).await?
            } else {
                let thumb = document
                    .thumbs()
                    .into_iter()
                    .max_by_key(|thumb| thumb.size())?;
                download_downloadable_bytes(client, &thumb, TELEGRAM_FAST_MEDIA_MAX_BYTES).await?
            }
        }
        Media::Document(document) => match fast_document_media_kind(&document)? {
            TelegramMediaKind::Photo => {
                download_downloadable_bytes(client, &document, TELEGRAM_FAST_MEDIA_MAX_BYTES)
                    .await?
            }
            _ => {
                let thumb = document
                    .thumbs()
                    .into_iter()
                    .max_by_key(|thumb| thumb.size())?;
                download_downloadable_bytes(client, &thumb, TELEGRAM_FAST_MEDIA_MAX_BYTES).await?
            }
        },
        _ => return None,
    };
    is_supported_raster_image(&bytes).then(|| ImageHandle::from_bytes(bytes))
}

/// Downloads a post's preview off the critical path and merges it back via a
/// follow-up `Loaded` event, so neither the live stream nor a backfilled page is
/// stalled on a file fetch. Bounded by `media_semaphore` so a media burst cannot
/// spawn unbounded concurrent downloads. On failure the post is re-delivered with
/// `failed_at_ms` set, so the card can stop showing a loading placeholder.
pub(super) fn spawn_fast_media_download(
    client: &Client,
    media_semaphore: &Arc<Semaphore>,
    output: &mpsc::Sender<TelegramFastFeedEvent>,
    channel: String,
    profile: TelegramChannelProfile,
    mut post: TelegramFeedPost,
    message: grammers_client::message::Message,
) {
    let client = client.clone();
    let media_semaphore = Arc::clone(media_semaphore);
    let mut output = output.clone();
    tokio::spawn(async move {
        let Ok(_permit) = media_semaphore.acquire_owned().await else {
            return;
        };
        let outcome = tokio::time::timeout(
            TELEGRAM_FAST_MEDIA_DOWNLOAD_TIMEOUT,
            download_fast_post_media(&client, &message),
        )
        .await;
        if let Some(media) = post.media.as_mut() {
            match outcome {
                Ok(Some(handle)) => media.handle = Some(handle),
                _ => media.failed_at_ms = Some(now_ms()),
            }
        }
        let _ = output
            .send(TelegramFastFeedEvent::Loaded(
                channel,
                Box::new(Ok(TelegramFeedPage {
                    profile,
                    posts: vec![post],
                })),
            ))
            .await;
    });
}

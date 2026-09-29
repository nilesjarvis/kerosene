use super::{
    TELEGRAM_FEED_FETCH_LIMIT, TelegramChannelProfile, TelegramFeedPage, TelegramFeedPost,
    TelegramFeedPostSource, TelegramMediaKind, TelegramPostMedia, is_supported_raster_image,
    normalize_public_channel_input, normalize_telegram_plain_text,
};
use crate::api::CLIENT;
use crate::app_time::now_ms;
use crate::helpers::{fallback_initials, text_excerpt};
use crate::network_activity::HttpRequestExt as _;
use chrono::{DateTime, Utc};
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use std::time::Duration;

const TELEGRAM_WEB_BASE: &str = "https://t.me/s/";
const TELEGRAM_USER_AGENT: &str =
    "Mozilla/5.0 (compatible; Kerosene Telegram Feed; +https://github.com)";
const TELEGRAM_FEED_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const TELEGRAM_FEED_MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const TELEGRAM_AVATAR_MAX_BODY_BYTES: usize = 512 * 1024;
const TELEGRAM_MEDIA_MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

pub(crate) async fn fetch_telegram_channel_posts(
    channel: String,
) -> Result<TelegramFeedPage, String> {
    let channel = normalize_public_channel_input(&channel)?;
    let url = format!("{TELEGRAM_WEB_BASE}{channel}");
    let request_started_ms = now_ms();
    let response = CLIENT
        .get(&url)
        .header(USER_AGENT, TELEGRAM_USER_AGENT)
        .timeout(TELEGRAM_FEED_REQUEST_TIMEOUT)
        .send_observed()
        .await
        .map_err(|e| format!("@{channel} request failed: {e}"))?;
    let status = response.status();
    let body_bytes = read_response_body_limited(
        response,
        TELEGRAM_FEED_MAX_BODY_BYTES,
        &format!("@{channel}"),
    )
    .await?;
    let fetched_at_ms = now_ms();
    let request_duration_ms = fetched_at_ms.saturating_sub(request_started_ms);
    let body = String::from_utf8_lossy(&body_bytes);

    if !status.is_success() {
        let preview = text_excerpt(&body, 160);
        return if preview.is_empty() {
            Err(format!("@{channel} request failed with HTTP {status}"))
        } else {
            Err(format!(
                "@{channel} request failed with HTTP {status}: {preview}"
            ))
        };
    }

    let profile = parse_telegram_channel_profile(&channel, &body);
    let posts = parse_telegram_channel_html(&channel, &body, TELEGRAM_FEED_FETCH_LIMIT)
        .into_iter()
        .map(|post| post.with_fetch_timing(request_started_ms, fetched_at_ms, request_duration_ms))
        .collect::<Vec<_>>();
    if posts.is_empty() {
        Err(format!(
            "@{channel} returned no public posts. Check that it is a public channel."
        ))
    } else {
        Ok(TelegramFeedPage { profile, posts })
    }
}

pub(crate) async fn fetch_telegram_avatar_bytes(
    channel: String,
    avatar_url: String,
) -> Result<Vec<u8>, String> {
    let channel = normalize_public_channel_input(&channel)?;
    fetch_telegram_image_bytes(
        &avatar_url,
        TELEGRAM_AVATAR_MAX_BODY_BYTES,
        &format!("@{channel} avatar"),
    )
    .await
}

pub(crate) async fn fetch_telegram_media_bytes(
    channel: String,
    message_id: u64,
    media_url: String,
) -> Result<Vec<u8>, String> {
    let channel = normalize_public_channel_input(&channel)?;
    fetch_telegram_image_bytes(
        &media_url,
        TELEGRAM_MEDIA_MAX_BODY_BYTES,
        &format!("@{channel}/{message_id} media"),
    )
    .await
}

async fn fetch_telegram_image_bytes(
    url: &str,
    max_body_bytes: usize,
    label: &str,
) -> Result<Vec<u8>, String> {
    let response = CLIENT
        .get(url)
        .header(USER_AGENT, TELEGRAM_USER_AGENT)
        .timeout(TELEGRAM_FEED_REQUEST_TIMEOUT)
        .send_observed()
        .await
        .map_err(|e| format!("{label} request failed: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("{label} request failed with HTTP {status}"));
    }
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let body = read_response_body_limited(response, max_body_bytes, label).await?;
    if !is_supported_raster_image(&body) {
        let content_type = content_type.as_deref().unwrap_or("unknown content type");
        return Err(format!(
            "{label} response was not a supported image: {content_type}"
        ));
    }

    Ok(body)
}

async fn read_response_body_limited(
    mut response: reqwest::Response,
    max_body_bytes: usize,
    label: &str,
) -> Result<Vec<u8>, String> {
    if response
        .content_length()
        .is_some_and(|len| len > max_body_bytes as u64)
    {
        return Err(format!(
            "{label} response was too large: more than {max_body_bytes} bytes"
        ));
    }

    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("{label} response read failed: {e}"))?
    {
        if body.len() + chunk.len() > max_body_bytes {
            return Err(format!(
                "{label} response was too large: more than {max_body_bytes} bytes"
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn parse_telegram_channel_profile(channel: &str, html: &str) -> TelegramChannelProfile {
    let channel = normalize_public_channel_input(channel).unwrap_or_else(|_| channel.to_string());
    let title = html_between(html, "tgme_channel_info_header_title", "</div>")
        .map(html_to_plain_text)
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| format!("@{channel}"));
    let photo_block = html
        .find("tgme_page_photo_image")
        .map(|start| {
            let end = html[start..]
                .find("</i>")
                .map(|offset| start + offset + 4)
                .unwrap_or_else(|| html.len().min(start + 2_000));
            &html[start..end]
        })
        .unwrap_or_default();
    let avatar_url = attr_value(photo_block, "src=\"").and_then(normalize_telegram_asset_url);
    let initials = attr_value(photo_block, "data-content=\"")
        .map(html_to_plain_text)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| fallback_initials(&title, &channel));

    TelegramChannelProfile {
        channel,
        title,
        initials,
        avatar_url,
        avatar_handle: None,
        avatar_loading_url: None,
        avatar_request_id: 0,
        avatar_failed_at_ms: None,
    }
}

fn parse_telegram_channel_html(channel: &str, html: &str, limit: usize) -> Vec<TelegramFeedPost> {
    let channel = normalize_public_channel_input(channel).unwrap_or_else(|_| channel.to_string());
    let mut posts = Vec::new();
    let mut cursor = 0;
    while let Some(relative_start) = html[cursor..].find("data-post=\"") {
        let post_start = cursor + relative_start;
        let block_end = html[post_start + 11..]
            .find("data-post=\"")
            .map(|offset| post_start + 11 + offset)
            .unwrap_or(html.len());
        let block = &html[post_start..block_end];
        if let Some(post) = parse_telegram_message_block(&channel, block) {
            posts.push(post);
        }
        cursor = block_end;
    }

    posts.sort_by(|left, right| {
        left.timestamp_ms
            .cmp(&right.timestamp_ms)
            .then_with(|| left.message_id.cmp(&right.message_id))
    });
    if posts.len() > limit {
        posts = posts.split_off(posts.len() - limit);
    }
    posts.reverse();
    posts
}

fn parse_telegram_message_block(channel: &str, block: &str) -> Option<TelegramFeedPost> {
    let data_post = attr_value(block, "data-post=\"")?;
    let (_, id) = data_post.rsplit_once('/')?;
    let message_id = id.parse::<u64>().ok()?;
    let datetime = attr_value(block, "datetime=\"")?;
    let timestamp_ms = DateTime::parse_from_rfc3339(datetime)
        .ok()?
        .with_timezone(&Utc)
        .timestamp_millis();
    let timestamp_ms = u64::try_from(timestamp_ms).ok()?;
    let media = parse_telegram_post_media(block);
    let caption = html_between(block, "tgme_widget_message_text js-message_text", "</div>")
        .map(html_to_plain_text)
        .filter(|caption| !caption.trim().is_empty());
    // A media-only post renders its preview instead of a placeholder string; only
    // posts with neither a caption nor a displayable preview fall back to a label,
    // and a post with nothing to show at all is dropped.
    let text = match caption {
        Some(caption) => caption,
        None if media.is_some() => String::new(),
        None => telegram_message_fallback_text(block),
    };
    if text.trim().is_empty() && media.is_none() {
        return None;
    }

    Some(TelegramFeedPost {
        channel: channel.to_string(),
        message_id,
        text,
        timestamp_ms,
        source: TelegramFeedPostSource::PublicPoll,
        received_at_ms: 0,
        applied_at_ms: 0,
        fetched_at_ms: 0,
        request_started_ms: 0,
        request_duration_ms: 0,
        first_seen_ms: 0,
        url: format!("https://t.me/{channel}/{message_id}"),
        ticker_mentions: Vec::new(),
        media,
    })
}

fn attr_value<'a>(block: &'a str, marker: &str) -> Option<&'a str> {
    let start = block.find(marker)? + marker.len();
    let end = block[start..].find('"')?;
    Some(&block[start..start + end])
}

fn html_between<'a>(block: &'a str, class_marker: &str, end_marker: &str) -> Option<&'a str> {
    let class_start = block.find(class_marker)?;
    let content_start = block[class_start..].find('>')? + class_start + 1;
    let content_end = block[content_start..].find(end_marker)? + content_start;
    Some(&block[content_start..content_end])
}

fn html_to_plain_text(html: &str) -> String {
    let normalized = html
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br>", "\n");
    let mut out = String::with_capacity(normalized.len());
    let mut in_tag = false;
    for ch in normalized.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }

    let plain_text = decode_html_entities(&out);
    normalize_telegram_plain_text(&plain_text)
}

fn normalize_telegram_asset_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        None
    } else if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        Some(trimmed.to_string())
    } else if trimmed.starts_with("//") {
        Some(format!("https:{trimmed}"))
    } else if trimmed.starts_with('/') {
        Some(format!("https://t.me{trimmed}"))
    } else {
        None
    }
}

/// Extracts the first displayable preview image from a public message block,
/// classifying it so the card can label media that has not loaded yet. Returns
/// `None` when the block has no extractable preview URL (the caller then keeps
/// the textual `[photo]`/`[video]` fallback instead).
fn parse_telegram_post_media(block: &str) -> Option<TelegramPostMedia> {
    // Stickers expose a static WebP preview via data-webp on t.me/s.
    if block.contains("tgme_widget_message_sticker")
        && let Some(url) = attr_value(block, "data-webp=\"")
            .and_then(|url| normalize_telegram_asset_url(&decode_html_entities(url.trim())))
            .or_else(|| telegram_media_background_url(block, "tgme_widget_message_sticker"))
    {
        return Some(TelegramPostMedia::from_url(TelegramMediaKind::Sticker, url));
    }
    // Round video messages carry their preview frame as a background image.
    if let Some(url) = telegram_media_background_url(block, "tgme_widget_message_roundvideo_thumb")
    {
        return Some(TelegramPostMedia::from_url(TelegramMediaKind::Video, url));
    }
    // Videos and GIFs both surface a still preview frame; t.me badges GIFs with a
    // "GIF" duration label (`<time ...>GIF</time>`). Matching the closing `</time>`
    // keeps a caption that merely contains the word "gif" from being misclassified.
    if let Some(url) = telegram_media_background_url(block, "tgme_widget_message_video_thumb") {
        let kind = if block.to_ascii_lowercase().contains(">gif</time>") {
            TelegramMediaKind::Gif
        } else {
            TelegramMediaKind::Video
        };
        return Some(TelegramPostMedia::from_url(kind, url));
    }
    // Plain photos.
    if let Some(url) = telegram_media_background_url(block, "tgme_widget_message_photo_wrap") {
        return Some(TelegramPostMedia::from_url(TelegramMediaKind::Photo, url));
    }
    None
}

/// Reads a `background-image:url(...)` value from the opening tag that carries
/// `class_marker`. Scanning only that tag keeps an element without an inline
/// style from borrowing a sibling element's preview URL.
fn telegram_media_background_url(block: &str, class_marker: &str) -> Option<String> {
    let class_start = block.find(class_marker)?;
    let tag_end = block[class_start..]
        .find('>')
        .map(|offset| class_start + offset)
        .unwrap_or(block.len());
    let tag = &block[class_start..tag_end];
    let background = tag.find("background-image")?;
    let url_open = tag[background..].find("url(")? + background + 4;
    // Decode entities before stripping quotes and locating the closing delimiter,
    // so an entity-encoded inner quote (`url(&#39;...&#39;)`) is handled like a
    // literal one instead of leaking into the extracted URL.
    let decoded = decode_html_entities(&tag[url_open..]);
    let value = decoded.trim_start_matches(['\'', '"', ' ']);
    let end = value.find([')', '\'', '"'])?;
    normalize_telegram_asset_url(value[..end].trim())
}

fn telegram_message_fallback_text(block: &str) -> String {
    if block.contains("tgme_widget_message_photo") {
        "[photo]".to_string()
    } else if block.contains("tgme_widget_message_video") {
        "[video]".to_string()
    } else if block.contains("tgme_widget_message_document") {
        "[file]".to_string()
    } else if block.contains("tgme_widget_message_poll") {
        "[poll]".to_string()
    } else if block.contains("tgme_widget_message_voice") {
        "[voice]".to_string()
    } else if block.contains("tgme_widget_message_roundvideo") {
        "[video message]".to_string()
    } else {
        String::new()
    }
}

fn decode_html_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        if let Some(end) = rest.find(';') {
            let entity = &rest[1..end];
            if let Some(decoded) = decode_entity(entity) {
                out.push(decoded);
                rest = &rest[end + 1..];
                continue;
            }
        }
        out.push('&');
        rest = &rest[1..];
    }
    out.push_str(rest);
    out
}

fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" | "#39" => Some('\''),
        "nbsp" => Some(' '),
        entity if entity.starts_with("#x") || entity.starts_with("#X") => {
            u32::from_str_radix(&entity[2..], 16)
                .ok()
                .and_then(char::from_u32)
        }
        entity if entity.starts_with('#') => {
            entity[1..].parse::<u32>().ok().and_then(char::from_u32)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;

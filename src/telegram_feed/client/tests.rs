use super::*;

mod http;

#[test]
fn parsing_preserves_equal_key_order_and_limit_edges() {
    let mut html = String::new();
    for (id, time, text) in [
        (9, "2026-05-31T18:00:00+00:00", "first duplicate"),
        (2, "2026-05-31T18:00:01+00:00", "newest"),
        (9, "2026-05-31T18:00:00+00:00", "last duplicate"),
        (8, "2026-05-31T18:00:00+00:00", "lower ID"),
        (1, "1960-01-01T00:00:00+00:00", "before epoch"),
    ] {
        html.push_str(&format!(
            r#"<div data-post="marketfeed/{id}"><div class="tgme_widget_message_text js-message_text">{text}</div><time datetime="{time}"></time></div>"#
        ));
    }

    let expected = ["newest", "last duplicate", "first duplicate", "lower ID"];
    for limit in [0, 1, 2, 3, 4, 10] {
        let posts = parse_telegram_channel_html(" @MarketFeed ", &html, limit);
        assert_eq!(
            posts
                .iter()
                .map(|post| post.text.as_str())
                .collect::<Vec<_>>(),
            expected[..limit.min(expected.len())]
        );
        assert!(posts.iter().all(|post| post.channel == "marketfeed"));
    }
}

#[test]
fn profile_parsing_preserves_unicode_entities_and_missing_metadata_fallbacks() {
    let html = r#"<div class="tgme_channel_info_header_title"><b>Énergie &amp; marchés</b></div><i class="tgme_page_photo_image" data-content="ÉM"><img src="//example.com/photo.jpg"></i>"#;
    let profile = parse_telegram_channel_profile(" @MarketFeed ", html);
    assert_eq!(profile.title, "Énergie & marchés");
    assert_eq!(profile.initials, "ÉM");
    assert_eq!(
        profile.avatar_url.as_deref(),
        Some("https://example.com/photo.jpg")
    );

    let profile = parse_telegram_channel_profile(
        "marketfeed",
        r#"<div class="tgme_channel_info_header_title">🔥</div><i class="tgme_page_photo_image" data-content=" "><img src="unsupported"></i>"#,
    );
    assert_eq!(profile.title, "@marketfeed");
    assert_eq!(
        profile.initials,
        fallback_initials("@marketfeed", "marketfeed")
    );
    assert!(profile.avatar_url.is_none());
}

const SAMPLE_HTML: &str = r#"
<div class="tgme_channel_info">
  <div class="tgme_channel_info_header">
    <i class="tgme_page_photo_image bgcolor3" data-content="MF"><img src="https://cdn4.telesco.pe/file/avatar.jpg"></i>
    <div class="tgme_channel_info_header_title"><span dir="auto">Market News Feed</span></div>
    <div class="tgme_channel_info_header_username"><a href="https://t.me/marketfeed">@marketfeed</a></div>
  </div>
</div>
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/10">
<div class="tgme_widget_message_text js-message_text" dir="auto">FIRST &amp; <b>FAST</b><br/>line two <a href="https://example.com">link</a></div>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/10"><time datetime="2026-05-31T17:50:14+00:00" class="time">17:50</time></a>
</div></div>
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/11">
<div class="tgme_widget_message_text js-message_text" dir="auto">SECOND &#39;POST&#39;</div>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/11"><time datetime="2026-05-31T18:00:00+00:00" class="time">18:00</time></a>
</div></div>
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/12">
<a class="tgme_widget_message_photo_wrap" href="https://t.me/marketfeed/12"><i class="tgme_widget_message_photo"></i></a>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/12"><time datetime="2026-05-31T18:01:00+00:00" class="time">18:01</time></a>
</div></div>
"#;

#[test]
fn parses_channel_profile_avatar_metadata() {
    let profile = parse_telegram_channel_profile("marketfeed", SAMPLE_HTML);

    assert_eq!(profile.channel, "marketfeed");
    assert_eq!(profile.title, "Market News Feed");
    assert_eq!(profile.initials, "MF");
    assert_eq!(
        profile.avatar_url.as_deref(),
        Some("https://cdn4.telesco.pe/file/avatar.jpg")
    );
}

#[test]
fn parses_public_channel_html_and_limits_latest_posts() {
    let posts = parse_telegram_channel_html("marketfeed", SAMPLE_HTML, 1);

    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].message_id, 12);
    assert_eq!(posts[0].text, "[photo]");
    assert_eq!(posts[0].url, "https://t.me/marketfeed/12");
}

#[test]
fn decodes_tags_breaks_and_entities() {
    let posts = parse_telegram_channel_html("marketfeed", SAMPLE_HTML, 10);
    let first = posts.iter().find(|post| post.message_id == 10).unwrap();

    assert_eq!(first.text, "FIRST & FAST\nline two link");
}

#[test]
fn strips_emoji_that_bundled_fonts_do_not_render() {
    let html = r#"
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/20">
<div class="tgme_widget_message_text js-message_text" dir="auto">🚨 BREAKING ⚡️ BTC pumps 🟢<br/>alpha 👨‍💻 desk</div>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/20"><time datetime="2026-05-31T18:02:00+00:00" class="time">18:02</time></a>
</div></div>
"#;
    let posts = parse_telegram_channel_html("marketfeed", html, 10);

    assert_eq!(posts[0].text, "BREAKING BTC pumps\nalpha desk");
}

#[test]
fn skips_messages_that_only_contain_stripped_emoji() {
    let html = r#"
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/21">
<div class="tgme_widget_message_text js-message_text" dir="auto">🔥🔥🔥</div>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/21"><time datetime="2026-05-31T18:03:00+00:00" class="time">18:03</time></a>
</div></div>
"#;
    let posts = parse_telegram_channel_html("marketfeed", html, 10);

    assert!(posts.is_empty());
}

#[test]
fn parses_media_only_posts_with_fallback_text() {
    let posts = parse_telegram_channel_html("marketfeed", SAMPLE_HTML, 10);
    let media_post = posts.iter().find(|post| post.message_id == 12).unwrap();

    // No preview URL is extractable from this block, so the post keeps the
    // textual placeholder and carries no displayable media.
    assert_eq!(media_post.text, "[photo]");
    assert!(media_post.media.is_none());
}

const MEDIA_HTML: &str = r#"
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/30">
<a class="tgme_widget_message_photo_wrap" style="background-image:url('https://cdn4.telesco.pe/file/photo30.jpg')" href="https://t.me/marketfeed/30"></a>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/30"><time datetime="2026-05-31T19:00:00+00:00" class="time">19:00</time></a>
</div></div>
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/31">
<div class="tgme_widget_message_text js-message_text" dir="auto">caption here</div>
<a class="tgme_widget_message_video_player blured" href="https://t.me/marketfeed/31"><i class="tgme_widget_message_video_thumb" style="background-image:url('https://cdn4.telesco.pe/file/video31.jpg')"></i><time class="message_video_duration js-message_video_duration">0:15</time></a>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/31"><time datetime="2026-05-31T19:01:00+00:00" class="time">19:01</time></a>
</div></div>
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/32">
<i class="tgme_widget_message_sticker" data-webp="https://cdn4.telesco.pe/file/sticker32.webp" style="width:128px;height:128px"></i>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/32"><time datetime="2026-05-31T19:02:00+00:00" class="time">19:02</time></a>
</div></div>
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/33">
<a class="tgme_widget_message_video_player" href="https://t.me/marketfeed/33"><i class="tgme_widget_message_video_thumb" style="background-image:url('https://cdn4.telesco.pe/file/gif33.jpg')"></i><time class="message_video_duration">GIF</time></a>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/33"><time datetime="2026-05-31T19:03:00+00:00" class="time">19:03</time></a>
</div></div>
"#;

#[test]
fn parses_attached_media_previews() {
    let posts = parse_telegram_channel_html("marketfeed", MEDIA_HTML, 10);

    let photo = posts.iter().find(|post| post.message_id == 30).unwrap();
    // A captionless photo renders its preview, so it carries no placeholder text.
    assert!(photo.text.is_empty());
    let photo_media = photo.media.as_ref().expect("photo media");
    assert_eq!(photo_media.kind, TelegramMediaKind::Photo);
    assert_eq!(
        photo_media.url.as_deref(),
        Some("https://cdn4.telesco.pe/file/photo30.jpg")
    );
    assert!(photo_media.handle.is_none());

    let video = posts.iter().find(|post| post.message_id == 31).unwrap();
    assert_eq!(video.text, "caption here");
    let video_media = video.media.as_ref().expect("video media");
    assert_eq!(video_media.kind, TelegramMediaKind::Video);
    assert_eq!(
        video_media.url.as_deref(),
        Some("https://cdn4.telesco.pe/file/video31.jpg")
    );

    let sticker = posts.iter().find(|post| post.message_id == 32).unwrap();
    let sticker_media = sticker.media.as_ref().expect("sticker media");
    assert_eq!(sticker_media.kind, TelegramMediaKind::Sticker);
    assert_eq!(
        sticker_media.url.as_deref(),
        Some("https://cdn4.telesco.pe/file/sticker32.webp")
    );

    let gif = posts.iter().find(|post| post.message_id == 33).unwrap();
    assert_eq!(
        gif.media.as_ref().expect("gif media").kind,
        TelegramMediaKind::Gif
    );
}

#[test]
fn extracts_background_image_url_with_entity_encoded_quotes() {
    let block = r#"<div data-post="marketfeed/40">
<a class="tgme_widget_message_photo_wrap" style="background-image:url(&#39;https://cdn4.telesco.pe/file/enc40.jpg&#39;)" href="https://t.me/marketfeed/40"></a>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/40"><time datetime="2026-05-31T20:00:00+00:00">20:00</time></a>
</div>"#;
    let posts = parse_telegram_channel_html("marketfeed", block, 10);

    let media = posts[0].media.as_ref().expect("photo media");
    assert_eq!(media.kind, TelegramMediaKind::Photo);
    assert_eq!(
        media.url.as_deref(),
        Some("https://cdn4.telesco.pe/file/enc40.jpg")
    );
}

#[test]
fn video_with_gif_caption_text_is_not_classified_as_gif() {
    // The caption merely contains the word "gif"; only the duration badge
    // (`<time>GIF</time>`) should drive GIF classification.
    let block = r#"<div data-post="marketfeed/41">
<div class="tgme_widget_message_text js-message_text" dir="auto">is this a gif or a video</div>
<a class="tgme_widget_message_video_player" href="https://t.me/marketfeed/41"><i class="tgme_widget_message_video_thumb" style="background-image:url('https://cdn4.telesco.pe/file/vid41.jpg')"></i><time class="message_video_duration">0:30</time></a>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/41"><time datetime="2026-05-31T20:01:00+00:00">20:01</time></a>
</div>"#;
    let posts = parse_telegram_channel_html("marketfeed", block, 10);

    assert_eq!(
        posts[0].media.as_ref().expect("video media").kind,
        TelegramMediaKind::Video
    );
}

#[tokio::test]
#[ignore]
async fn live_marketfeed_fetches_posts() {
    let page = fetch_telegram_channel_posts("@marketfeed".to_string())
        .await
        .expect("@marketfeed should fetch");
    let posts = page.posts;

    assert!(!posts.is_empty());
    assert!(posts.len() <= TELEGRAM_FEED_FETCH_LIMIT);
    assert!(posts.iter().all(|post| post.channel == "marketfeed"));
    assert!(posts.iter().all(|post| post.fetched_at_ms > 0));
    assert!(posts.iter().all(|post| post.request_started_ms > 0));
    assert_eq!(page.profile.channel, "marketfeed");
}

#[tokio::test]
#[ignore]
async fn live_marketfeed_fetches_avatar() {
    let page = fetch_telegram_channel_posts("@marketfeed".to_string())
        .await
        .expect("@marketfeed should fetch");
    let avatar_url = page
        .profile
        .avatar_url
        .expect("@marketfeed should expose an avatar URL");
    let avatar = fetch_telegram_avatar_bytes("@marketfeed".to_string(), avatar_url)
        .await
        .expect("@marketfeed avatar should fetch");

    assert!(!avatar.is_empty());
    assert!(avatar.len() <= TELEGRAM_AVATAR_MAX_BODY_BYTES);
}

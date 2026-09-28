use super::*;
use crate::config::KeroseneConfig;
use crate::x_feed::XFeedPost;

fn test_post(author_id: &str, image_url: Option<&str>) -> XFeedPost {
    XFeedPost {
        id: "1".to_string(),
        author_id: Some(author_id.to_string()),
        author_name: "Alice Example".to_string(),
        author_username: "alice".to_string(),
        author_profile_image_url: image_url.map(str::to_string),
        text: "test post".to_string(),
        created_at_ms: 1_000,
        received_at_ms: 2_000,
        url: "https://example.com/post/1".to_string(),
    }
}

fn test_page(posts: Vec<XFeedPost>) -> XFeedPage {
    XFeedPage {
        source: XFeedSource::Following,
        posts,
        newest_id: None,
        rate_limited_until_ms: None,
    }
}

#[test]
fn open_refresh_includes_canvas_x_feed_instances() {
    let (mut terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let id = 17;
    terminal
        .x_feed
        .instances
        .insert(id, XFeedInstance::new(id, XFeedSource::Following));
    terminal.insert_test_canvas_pane(7, PaneKind::XFeed(id));

    let _task = terminal.request_x_feed_open_refresh(true);

    assert_eq!(
        terminal.x_feed.instances[&id].last_error.as_deref(),
        Some("Paste an X OAuth 2.0 user access token")
    );
}

#[test]
fn profile_fetches_dedupe_authors_and_update_metadata_without_an_image() {
    let (mut terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let original = test_post("42", Some("https://example.com/alice.jpg"));
    let mut renamed = original.clone();
    renamed.author_username = "alice_updated".to_string();
    renamed.author_name = "New Name".to_string();
    renamed.author_profile_image_url = None;
    let page = test_page(vec![
        original.clone(),
        original.clone(),
        renamed,
        test_post("43", original.author_profile_image_url.as_deref()),
        test_post("44", None),
    ]);

    assert_eq!(terminal.schedule_x_profile_image_fetches(&page).units(), 2);

    let profile = &terminal.x_feed.author_profiles[&original.author_profile_key()];
    assert_eq!(profile.author_id.as_deref(), Some("42"));
    assert_eq!(profile.username, "alice_updated");
    assert_eq!(profile.name, "New Name");
    assert_eq!(profile.initials, "NN");
    assert_eq!(profile.profile_image_url, original.author_profile_image_url);
    assert_eq!(profile.image_loading_url, original.author_profile_image_url);
    assert_eq!(profile.image_request_id, 1);
    assert_eq!(terminal.x_feed.next_profile_image_request_id, 2);
    assert_eq!(terminal.x_feed.author_profiles.len(), 3);
    let no_image = &terminal.x_feed.author_profiles["id:44"];
    assert!(no_image.profile_image_url.is_none());
    assert!(no_image.image_loading_url.is_none());
    assert_eq!(no_image.image_request_id, 0);
}

#[test]
fn unchanged_or_missing_image_url_preserves_cached_pending_and_failed_profiles() {
    let (mut terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let original = test_post("42", Some("https://example.com/alice.jpg"));
    let key = original.author_profile_key();

    for state in 0..3 {
        let mut expected = XAuthorProfile::from_post(&original);
        match state {
            0 => expected.image_handle = Some(ImageHandle::from_rgba(1, 1, vec![0; 4])),
            1 => {
                expected.image_loading_url = original.author_profile_image_url.clone();
                expected.image_request_id = 7;
            }
            _ => expected.image_failed_at_ms = Some(u64::MAX),
        }
        terminal
            .x_feed
            .author_profiles
            .insert(key.clone(), expected.clone());
        for image_url in [original.author_profile_image_url.clone(), None] {
            let mut post = original.clone();
            post.author_profile_image_url = image_url;
            post.author_name = "Updated Name".to_string();
            expected.name.clone_from(&post.author_name);
            expected.initials = post.author_initials();

            assert_eq!(
                terminal
                    .schedule_x_profile_image_fetches(&test_page(vec![post]))
                    .units(),
                0
            );
            assert_eq!(terminal.x_feed.author_profiles[&key], expected);
        }
    }
    assert_eq!(terminal.x_feed.next_profile_image_request_id, 0);
}

#[test]
fn changed_image_rejects_stale_results_and_retries_after_backoff() {
    let (mut terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let mut post = test_post("42", Some("https://example.com/old.jpg"));
    let key = post.author_profile_key();
    assert_eq!(
        terminal
            .schedule_x_profile_image_fetches(&test_page(vec![post.clone()]))
            .units(),
        1
    );
    let old_request_id = terminal.x_feed.author_profiles[&key].image_request_id;
    post.author_profile_image_url = Some("https://example.com/new.jpg".to_string());
    let page = test_page(vec![post.clone()]);
    assert_eq!(terminal.schedule_x_profile_image_fetches(&page).units(), 1);
    let pending = terminal.x_feed.author_profiles[&key].clone();
    assert_ne!(pending.image_request_id, old_request_id);
    assert_eq!(pending.profile_image_url, post.author_profile_image_url);
    assert_eq!(pending.image_loading_url, post.author_profile_image_url);
    assert!(pending.image_handle.is_none());
    assert!(pending.image_failed_at_ms.is_none());

    terminal.handle_x_profile_image_loaded(old_request_id, Ok(vec![1, 2, 3]));
    terminal.handle_x_profile_image_loaded(old_request_id, Err("stale".to_string()));
    assert_eq!(terminal.x_feed.author_profiles[&key], pending);

    terminal.handle_x_profile_image_loaded(pending.image_request_id, Err("failed".to_string()));
    let failed = &terminal.x_feed.author_profiles[&key];
    assert!(failed.image_handle.is_none());
    assert!(failed.image_loading_url.is_none());
    assert_eq!(failed.image_request_id, 0);
    assert!(failed.image_failed_at_ms.is_some());
    assert_eq!(terminal.schedule_x_profile_image_fetches(&page).units(), 0);

    terminal
        .x_feed
        .author_profiles
        .get_mut(&key)
        .expect("existing profile")
        .image_failed_at_ms =
        Some(TradingTerminal::now_ms().saturating_sub(X_PROFILE_IMAGE_RETRY_BACKOFF_MS));
    assert_eq!(terminal.schedule_x_profile_image_fetches(&page).units(), 1);
    let retry = terminal.x_feed.author_profiles[&key].image_request_id;
    assert!(retry > pending.image_request_id);
    assert!(
        terminal.x_feed.author_profiles[&key]
            .image_failed_at_ms
            .is_none()
    );
    terminal.handle_x_profile_image_loaded(retry, Ok(vec![1, 2, 3]));
    let loaded = terminal.x_feed.author_profiles[&key].clone();
    assert!(loaded.image_handle.is_some());
    assert!(loaded.image_loading_url.is_none());
    assert_eq!(loaded.image_request_id, 0);
    assert!(loaded.image_failed_at_ms.is_none());
    assert_eq!(terminal.schedule_x_profile_image_fetches(&page).units(), 0);
    terminal.handle_x_profile_image_loaded(retry, Err("duplicate result".to_string()));
    assert_eq!(terminal.x_feed.author_profiles[&key], loaded);

    post.author_profile_image_url = Some("https://example.com/third.jpg".to_string());
    assert_eq!(
        terminal
            .schedule_x_profile_image_fetches(&test_page(vec![post]))
            .units(),
        1
    );
    assert!(terminal.x_feed.author_profiles[&key].image_handle.is_none());
}

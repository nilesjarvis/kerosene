use super::*;

#[test]
fn contexts_loaded_replaces_context_map_and_records_fetch_time() {
    let mut loading = true;
    let mut last_fetch_ms = None;
    let mut contexts = HashMap::from([("OLD".to_string(), context(1.0))]);
    let mut status = Some(("previous".to_string(), true));

    apply_contexts_loaded(
        &mut loading,
        &mut last_fetch_ms,
        &mut contexts,
        &mut status,
        42,
        Ok(HashMap::from([("BTC".to_string(), context(2.0))]).into()),
    );

    assert!(!loading);
    assert_eq!(last_fetch_ms, Some(42));
    assert!(!contexts.contains_key("OLD"));
    assert_eq!(contexts.get("BTC").map(|ctx| ctx.day_vlm), Some(Some(2.0)));
    assert_eq!(status, Some(("previous".to_string(), true)));
}

#[test]
fn contexts_loaded_error_keeps_existing_contexts_without_marking_fresh() {
    let mut loading = true;
    let mut last_fetch_ms = None;
    let mut contexts = HashMap::from([("BTC".to_string(), context(1.0))]);
    let mut status = None;

    apply_contexts_loaded(
        &mut loading,
        &mut last_fetch_ms,
        &mut contexts,
        &mut status,
        50,
        Err("network".to_string()),
    );

    assert!(!loading);
    assert_eq!(last_fetch_ms, None);
    assert_eq!(contexts.get("BTC").map(|ctx| ctx.day_vlm), Some(Some(1.0)));
    assert_eq!(
        status,
        Some((
            "Watchlist context refresh failed: network".to_string(),
            true
        ))
    );
}

#[test]
fn contexts_loaded_error_redacts_sensitive_status_details() {
    let mut loading = true;
    let mut last_fetch_ms = None;
    let mut contexts = HashMap::new();
    let mut status = None;

    apply_contexts_loaded(
        &mut loading,
        &mut last_fetch_ms,
        &mut contexts,
        &mut status,
        50,
        Err("provider rejected api_key=key-secret cursor=cursor-secret".to_string()),
    );

    let (message, is_error) = status.expect("status");
    assert!(is_error);
    assert!(message.contains("api_key=<redacted>"));
    assert!(message.contains("cursor=<redacted>"));
    assert!(!message.contains("key-secret"));
    assert!(!message.contains("cursor-secret"));
}

#[test]
fn contexts_loaded_partial_success_reports_error_without_discarding_data() {
    let mut loading = true;
    let mut last_fetch_ms = None;
    let mut contexts = HashMap::new();
    let mut status = None;

    apply_contexts_loaded(
        &mut loading,
        &mut last_fetch_ms,
        &mut contexts,
        &mut status,
        50,
        Ok(crate::api::WatchlistContextsResponse {
            contexts: HashMap::from([("@107".to_string(), context(3.0))]),
            partial_errors: vec!["HIP-3 dex xyz: HTTP 503".to_string()],
        }),
    );

    assert_eq!(last_fetch_ms, Some(50));
    assert_eq!(
        contexts.get("@107").and_then(|context| context.day_vlm),
        Some(3.0)
    );
    assert_eq!(
        status,
        Some((
            "Watchlist context refresh partially failed: HIP-3 dex xyz: HTTP 503".to_string(),
            true
        ))
    );

    apply_contexts_loaded(
        &mut loading,
        &mut last_fetch_ms,
        &mut contexts,
        &mut status,
        60,
        Ok(HashMap::from([("@107".to_string(), context(4.0))]).into()),
    );
    assert_eq!(status, None, "a complete refresh clears partial warning");
}

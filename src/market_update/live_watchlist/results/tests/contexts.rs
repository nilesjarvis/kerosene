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
        Ok(HashMap::from([("BTC".to_string(), context(2.0))])),
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

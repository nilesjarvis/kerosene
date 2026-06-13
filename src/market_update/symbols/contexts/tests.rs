use crate::api::WatchlistContext;

use super::*;
use std::collections::HashMap;

fn context(day_vlm: f64) -> WatchlistContext {
    WatchlistContext {
        funding: None,
        prev_day_px: None,
        day_vlm: Some(day_vlm),
    }
}

#[test]
fn contexts_loaded_success_replaces_cache_and_clears_status() {
    let mut loading = true;
    let mut last_fetch_ms = None;
    let mut contexts = HashMap::from([
        ("BTC".to_string(), context(1.0)),
        ("SOL".to_string(), context(2.0)),
    ]);
    let mut status = Some(("old error".to_string(), true));

    apply_contexts_loaded(
        &mut loading,
        &mut last_fetch_ms,
        &mut contexts,
        &mut status,
        42,
        Ok(HashMap::from([
            ("BTC".to_string(), context(3.0)),
            ("ETH".to_string(), context(4.0)),
        ])),
    );

    assert!(!loading);
    assert_eq!(last_fetch_ms, Some(42));
    assert_eq!(contexts.get("BTC").map(|ctx| ctx.day_vlm), Some(Some(3.0)));
    assert!(!contexts.contains_key("SOL"));
    assert_eq!(contexts.get("ETH").map(|ctx| ctx.day_vlm), Some(Some(4.0)));
    assert_eq!(status, None);
}

#[test]
fn contexts_loaded_error_keeps_cache_and_fetch_timestamp() {
    let mut loading = true;
    let mut last_fetch_ms = Some(20);
    let mut contexts = HashMap::from([("BTC".to_string(), context(1.0))]);
    let mut status = None;

    apply_contexts_loaded(
        &mut loading,
        &mut last_fetch_ms,
        &mut contexts,
        &mut status,
        42,
        Err("rate limited".to_string()),
    );

    assert!(!loading);
    assert_eq!(last_fetch_ms, Some(20));
    assert_eq!(contexts.get("BTC").map(|ctx| ctx.day_vlm), Some(Some(1.0)));
    assert_eq!(
        status,
        Some(("24h volume refresh failed: rate limited".to_string(), true))
    );
}

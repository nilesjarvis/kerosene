use super::storage::save_json;
use super::*;
use std::fs;

pub(super) fn test_cache_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("kerosene-api-cache-{name}-{nanos}"))
}

#[test]
fn watchlist_context_cache_requires_all_symbols_fresh() {
    let root = test_cache_dir("contexts");
    let now = 1_000_000;
    let btc = WatchlistContext {
        funding: Some(0.001),
        prev_day_px: Some(100.0),
        mark_px: Some(110.0),
        day_vlm: Some(1_000.0),
        open_interest_notional: None,
    };
    save_json(
        &root,
        "watchlist_contexts",
        &["BTC".to_string()],
        now,
        None,
        &btc,
    )
    .expect("context save succeeds");

    let symbols = vec!["BTC".to_string()];
    let fresh = load_fresh_watchlist_contexts_from_dir(&root, &symbols, now + 1_000)
        .expect("context load succeeds")
        .expect("context should be fresh");
    assert!(fresh.contains_key("BTC"));
    assert_eq!(
        fresh.get("BTC").and_then(|context| context.mark_px),
        Some(110.0)
    );

    let stale = load_fresh_watchlist_contexts_from_dir(
        &root,
        &symbols,
        now + WATCHLIST_CONTEXT_FRESH_MS + 1,
    )
    .expect("context load succeeds");
    assert!(stale.is_none());

    let missing_symbols = vec!["BTC".to_string(), "ETH".to_string()];
    let missing = load_fresh_watchlist_contexts_from_dir(&root, &missing_symbols, now + 1_000)
        .expect("context load succeeds");
    assert!(missing.is_none());

    let _ = fs::remove_dir_all(root);
}

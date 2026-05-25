use super::*;

#[test]
fn history_loaded_records_all_requested_symbols_and_merges_history() {
    let mut loading = true;
    let mut loaded_at = HashMap::new();
    let mut history = HashMap::from([("BTC".to_string(), (1.0, 2.0, 3.0))]);
    let mut status = None;

    apply_history_loaded(
        &mut loading,
        &mut loaded_at,
        &mut history,
        &mut status,
        vec!["BTC".to_string(), "ETH".to_string()],
        60,
        Ok(HashMap::from([
            ("BTC".to_string(), (4.0, 5.0, 6.0)),
            ("ETH".to_string(), (7.0, 8.0, 9.0)),
        ])),
    );

    assert!(!loading);
    assert_eq!(loaded_at.get("BTC").copied(), Some(60));
    assert_eq!(loaded_at.get("ETH").copied(), Some(60));
    assert_eq!(history.get("BTC").copied(), Some((4.0, 5.0, 6.0)));
    assert_eq!(history.get("ETH").copied(), Some((7.0, 8.0, 9.0)));
    assert_eq!(status, None);
}

#[test]
fn history_loaded_error_still_records_requested_symbols() {
    let mut loading = true;
    let mut loaded_at = HashMap::new();
    let mut history = HashMap::from([("BTC".to_string(), (1.0, 2.0, 3.0))]);
    let mut status = None;

    apply_history_loaded(
        &mut loading,
        &mut loaded_at,
        &mut history,
        &mut status,
        vec!["ETH".to_string()],
        70,
        Err("timeout".to_string()),
    );

    assert!(!loading);
    assert_eq!(loaded_at.get("ETH").copied(), Some(70));
    assert_eq!(history.get("BTC").copied(), Some((1.0, 2.0, 3.0)));
    assert_eq!(
        status,
        Some((
            "Watchlist history refresh failed: timeout".to_string(),
            true
        ))
    );
}

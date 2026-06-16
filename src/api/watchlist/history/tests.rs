use super::{finish_symbol_history, history_baselines, screener_history_baselines};
use std::collections::HashMap;

fn candle(open_time: u64, open: f64) -> crate::api::Candle {
    crate::api::Candle::test_ohlcv(open_time, open_time + 60_000, [open, open, open, open], 1.0)
}

#[test]
fn history_baselines_are_unknown_for_empty_or_nonfinite_history() {
    assert_eq!(history_baselines(Vec::new(), 100_000), None);
    assert_eq!(
        history_baselines(vec![candle(1_000, f64::NAN)], 100_000),
        None
    );
}

#[test]
fn history_baselines_use_latest_candle_before_each_target() {
    let now = 65 * 60 * 1000;
    let candles = vec![
        candle(0, 1.0),
        candle(5 * 60 * 1000, 2.0),
        candle(35 * 60 * 1000, 3.0),
        candle(60 * 60 * 1000, 4.0),
    ];

    assert_eq!(history_baselines(candles, now), Some((4.0, 3.0, 2.0)));
}

#[test]
fn screener_history_baselines_use_fifteen_minute_and_one_hour_targets() {
    let now = 65 * 60 * 1000;
    let candles = vec![
        candle(0, 1.0),
        candle(5 * 60 * 1000, 2.0),
        candle(50 * 60 * 1000, 3.0),
        candle(60 * 60 * 1000, 4.0),
    ];

    assert_eq!(screener_history_baselines(candles, now), Some((3.0, 2.0)));
}

#[test]
fn watchlist_history_finish_errors_when_all_requested_fetches_fail() {
    let result: Result<HashMap<String, (f64, f64, f64)>, String> = finish_symbol_history(
        HashMap::new(),
        2,
        2,
        Some("network down".to_string()),
        Some("No watchlist history available"),
    );

    assert_eq!(result, Err("network down".to_string()));
}

#[test]
fn watchlist_history_finish_allows_empty_success_without_fetch_failures() {
    let result: Result<HashMap<String, (f64, f64, f64)>, String> = finish_symbol_history(
        HashMap::new(),
        2,
        0,
        None,
        Some("No watchlist history available"),
    );

    assert_eq!(result, Ok(HashMap::new()));
}

#[test]
fn watchlist_history_finish_keeps_partial_success_when_some_fetches_fail() {
    let history = HashMap::from([("BTC".to_string(), (1.0, 2.0, 3.0))]);
    let result = finish_symbol_history(
        history.clone(),
        2,
        1,
        Some("ETH timeout".to_string()),
        Some("No watchlist history available"),
    );

    assert_eq!(result, Ok(history));
}

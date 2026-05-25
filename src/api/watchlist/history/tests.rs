use super::history_baselines;
use crate::api::Candle;

fn candle(open_time: u64, open: f64) -> Candle {
    Candle {
        open_time,
        close_time: open_time + 60_000,
        open,
        high: open,
        low: open,
        close: open,
        volume: 1.0,
    }
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

use super::*;
use crate::api::Candle;
use crate::helpers::assert_close_tight as assert_close;
use crate::spaghetti::Series;
use iced::Color;

fn candle_at(open_time: u64, close: f64) -> Candle {
    Candle::test_ohlcv(
        open_time,
        open_time + 59_999,
        [close, close + 1.0, close - 1.0, close],
        1.0,
    )
}

fn series(symbol: &str, candles: Vec<Candle>) -> Series {
    Series {
        symbol: symbol.to_string(),
        display: symbol.to_string(),
        candles,
        color: Color::WHITE,
        loaded: true,
    }
}

#[test]
fn pair_ratio_reset_keeps_intraday_default_window() {
    let chart_w = 720.0;
    let mut a_candles = Vec::new();
    let mut b_candles = Vec::new();
    for idx in 0..48 {
        let ts = idx * 3_600_000;
        a_candles.push(candle_at(ts, 10.0));
        b_candles.push(candle_at(ts, 20.0));
    }
    let a = series("HYPE", a_candles);
    let b = series("BTC", b_candles);

    let Some(px) = pair_ratio_reset_px_per_ms(chart_w, &[&a, &b]) else {
        panic!("reset px");
    };

    assert_close(px, DEFAULT_PX_PER_MS);
}

#[test]
fn pair_ratio_reset_fits_high_timeframe_overlap() {
    let chart_w = 720.0;
    let day_ms = 86_400_000;
    let mut a_candles = Vec::new();
    let mut b_candles = Vec::new();
    for idx in 0..120 {
        let ts = idx * day_ms;
        a_candles.push(candle_at(ts, 10.0));
        b_candles.push(candle_at(ts, 20.0));
    }
    let a = series("HYPE", a_candles);
    let b = series("BTC", b_candles);

    let Some(px) = pair_ratio_reset_px_per_ms(chart_w, &[&a, &b]) else {
        panic!("reset px");
    };
    let visible_days = chart_w as f64 / px / day_ms as f64;

    assert!(
        visible_days >= 94.0,
        "expected at least 94 visible days, got {visible_days}"
    );
    assert!(
        visible_days <= 96.0,
        "expected at most 96 visible days, got {visible_days}"
    );
}

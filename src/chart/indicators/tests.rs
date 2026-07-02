use crate::api::Candle;

use super::*;

#[test]
fn sma_uses_sliding_window() {
    let candles = vec![
        Candle::test_flat(1_000, 10.0),
        Candle::test_flat(2_000, 20.0),
        Candle::test_flat(3_000, 30.0),
        Candle::test_flat(4_000, 40.0),
    ];

    assert_eq!(
        calculate_sma(&candles, 3),
        vec![(3_000, 20.0), (4_000, 30.0)]
    );
}

#[test]
fn ema_seeds_from_initial_sma() {
    let candles = vec![
        Candle::test_flat(1_000, 10.0),
        Candle::test_flat(2_000, 20.0),
        Candle::test_flat(3_000, 30.0),
        Candle::test_flat(4_000, 40.0),
    ];

    assert_eq!(
        calculate_ema(&candles, 3),
        vec![(3_000, 20.0), (4_000, 30.0)]
    );
}

#[test]
fn zero_period_returns_empty_series() {
    assert!(calculate_sma(&[Candle::test_flat(1_000, 10.0)], 0).is_empty());
    assert!(calculate_ema(&[Candle::test_flat(1_000, 10.0)], 0).is_empty());
}

fn ohlc(open_time: u64, open: f64, high: f64, low: f64, close: f64) -> Candle {
    Candle {
        open_time,
        close_time: open_time + 999,
        open,
        high,
        low,
        close,
        volume: 1.0,
    }
}

fn rising_green(i: usize) -> Candle {
    let close = 100.0 + i as f64;
    ohlc(
        i as u64 * 1_000,
        close - 0.5,
        close + 1.0,
        close - 1.5,
        close,
    )
}

fn falling_red(i: usize) -> Candle {
    let close = 100.0 - i as f64;
    ohlc(
        i as u64 * 1_000,
        close + 0.5,
        close + 1.5,
        close - 1.0,
        close,
    )
}

#[test]
fn leledc_bearish_exhaustion_sets_resistance_at_bar_high() {
    let mut candles: Vec<Candle> = (0..15).map(rising_green).collect();
    // Red bar topping every prior high after 12 closes above the close 4 bars back.
    candles.push(ohlc(15_000, 116.0, 120.0, 114.0, 114.5));

    let levels = calculate_leledc(&candles, 40, 10);

    assert_eq!(levels.signals, vec![(15, LeledcSignal::Bearish)]);
    assert_eq!(
        levels.resistance,
        vec![LeledcLevel {
            start: 15,
            end: 15,
            price: 120.0,
        }]
    );
    assert!(levels.support.is_empty());
}

#[test]
fn leledc_bullish_exhaustion_sets_support_at_bar_low() {
    let mut candles: Vec<Candle> = (0..15).map(falling_red).collect();
    // Green bar undercutting every prior low after 12 closes below the close 4 bars back.
    candles.push(ohlc(15_000, 84.0, 86.0, 80.0, 85.5));

    let levels = calculate_leledc(&candles, 40, 10);

    assert_eq!(levels.signals, vec![(15, LeledcSignal::Bullish)]);
    assert_eq!(
        levels.support,
        vec![LeledcLevel {
            start: 15,
            end: 15,
            price: 80.0,
        }]
    );
    assert!(levels.resistance.is_empty());
}

#[test]
fn leledc_new_exhaustion_ends_previous_level_one_bar_early() {
    let mut candles: Vec<Candle> = (0..15).map(rising_green).collect();
    candles.push(ohlc(15_000, 116.0, 120.0, 114.0, 114.5));
    candles.extend((16..27).map(rising_green));
    candles.push(ohlc(27_000, 128.0, 131.0, 126.0, 126.5));

    let levels = calculate_leledc(&candles, 40, 10);

    assert_eq!(
        levels.signals,
        vec![(15, LeledcSignal::Bearish), (27, LeledcSignal::Bearish)]
    );
    assert_eq!(
        levels.resistance,
        vec![
            LeledcLevel {
                start: 15,
                end: 26,
                price: 120.0,
            },
            LeledcLevel {
                start: 27,
                end: 27,
                price: 131.0,
            },
        ]
    );
}

#[test]
fn leledc_requires_swing_extreme_to_fire() {
    let mut candles: Vec<Candle> = (0..15).map(rising_green).collect();
    // Red bar with enough directional closes but no 40-bar high.
    candles.push(ohlc(15_000, 114.9, 114.95, 113.9, 114.0));

    let levels = calculate_leledc(&candles, 40, 10);

    assert!(levels.signals.is_empty());
    assert!(levels.resistance.is_empty());
    assert!(levels.support.is_empty());
}

#[test]
fn leledc_degenerate_inputs_return_empty_levels() {
    assert_eq!(calculate_leledc(&[], 40, 10), LeledcLevels::default());
    assert_eq!(
        calculate_leledc(&[Candle::test_flat(1_000, 10.0)], 0, 10),
        LeledcLevels::default()
    );
}

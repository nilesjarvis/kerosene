use super::*;
use iced::advanced::renderer::Headless;
use iced::{Font, Pixels, Renderer, Size};

fn push(chart: &mut CandlestickChart, secondary: bool, candle: Candle) -> CandlePushResult {
    if secondary {
        chart.push_secondary_candle(candle)
    } else {
        chart.push_candle(candle)
    }
}

fn series(chart: &CandlestickChart, secondary: bool) -> &[Candle] {
    if secondary {
        &chart
            .secondary_series
            .as_ref()
            .expect("secondary series")
            .candles
    } else {
        &chart.candles
    }
}

fn candle_bits(candles: &[Candle]) -> Vec<[u64; 7]> {
    candles
        .iter()
        .map(|candle| {
            [
                candle.open_time,
                candle.close_time,
                candle.open.to_bits(),
                candle.high.to_bits(),
                candle.low.to_bits(),
                candle.close.to_bits(),
                candle.volume.to_bits(),
            ]
        })
        .collect()
}

#[tokio::test]
async fn realtime_candle_admission_preserves_series_status_and_cache_policy() {
    let renderer = Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let first = Candle::test_price(60_000, 10.0);
    let tail = Candle::test_price(120_000, 20.0);
    let replacement = Candle::test_ohlcv(120_000, 180_001, [-2.0, -1.0, -4.0, -3.0], 0.0);
    let next = Candle::test_flat(180_000, -0.0);
    let invalid = Candle {
        close: f64::NAN,
        ..tail.clone()
    };
    let invalid_old = Candle {
        volume: -1.0,
        ..first.clone()
    };
    let initial = vec![first.clone(), tail.clone()];
    let cases = [
        (
            vec![],
            invalid.clone(),
            CandlePushResult::RejectedInvalid,
            vec![],
        ),
        (
            vec![],
            first.clone(),
            CandlePushResult::Appended,
            vec![first.clone()],
        ),
        (
            initial.clone(),
            invalid,
            CandlePushResult::RejectedInvalid,
            initial.clone(),
        ),
        (
            initial.clone(),
            invalid_old,
            CandlePushResult::RejectedInvalid,
            initial.clone(),
        ),
        (
            initial.clone(),
            first.clone(),
            CandlePushResult::RejectedOutOfOrder,
            initial.clone(),
        ),
        (
            initial.clone(),
            tail.clone(),
            CandlePushResult::Updated,
            initial.clone(),
        ),
        (
            initial.clone(),
            replacement.clone(),
            CandlePushResult::Updated,
            vec![first.clone(), replacement],
        ),
        (
            initial.clone(),
            next.clone(),
            CandlePushResult::Appended,
            vec![first.clone(), tail.clone(), next],
        ),
    ];
    let size = Size::new(10.0, 10.0);
    for secondary in [false, true] {
        for (initial, incoming, expected_result, expected) in &cases {
            let mut chart = CandlestickChart::new(1);
            chart.set_secondary_series_identity("SYNTH".into(), "Synthetic".into());
            chart.candles = initial.clone();
            chart.secondary_series.as_mut().expect("secondary").candles = initial.clone();
            chart.status = ChartStatus::Error("retained status".into());
            let _ = chart.candle_cache.draw(&renderer, size, |_| {});

            let result = push(&mut chart, secondary, incoming.clone());

            assert_eq!(result, *expected_result);
            assert_eq!(
                candle_bits(series(&chart, secondary)),
                candle_bits(expected)
            );
            assert_eq!(
                candle_bits(series(&chart, !secondary)),
                candle_bits(initial)
            );
            assert!(
                matches!(&chart.status, ChartStatus::Error(error) if error == "retained status")
            );
            let identity = chart.secondary_series.as_ref().expect("secondary");
            assert_eq!(
                (&*identity.symbol_key, &*identity.symbol_label),
                ("SYNTH", "Synthetic")
            );
            let mut rebuilt = false;
            let _ = chart.candle_cache.draw(&renderer, size, |_| rebuilt = true);
            let applied = matches!(
                expected_result,
                CandlePushResult::Updated | CandlePushResult::Appended
            );
            assert_eq!(result.applied(), applied);
            assert_eq!(
                result.appended(),
                *expected_result == CandlePushResult::Appended
            );
            assert_eq!(rebuilt, applied);
        }
    }

    let mut chart = CandlestickChart::new(1);
    chart.set_candles(initial.clone());
    for incoming in [
        first,
        Candle {
            volume: -1.0,
            ..tail
        },
    ] {
        let _ = chart.candle_cache.draw(&renderer, size, |_| {});
        assert_eq!(
            chart.push_secondary_candle(incoming),
            CandlePushResult::RejectedInvalid
        );
        let mut rebuilt = false;
        let _ = chart.candle_cache.draw(&renderer, size, |_| rebuilt = true);
        assert!(!rebuilt);
        assert!(chart.secondary_series.is_none());
        assert!(matches!(chart.status, ChartStatus::Loaded));
        assert_eq!(candle_bits(&chart.candles), candle_bits(&initial));
    }
}

#[test]
fn realtime_candle_history_is_trimmed_only_on_append() {
    for secondary in [false, true] {
        for len in [MAX_CHART_CANDLES, MAX_CHART_CANDLES + 3] {
            let initial: Vec<_> = (1..=len)
                .map(|index| Candle::test_price(index as u64 * 60_000, index as f64))
                .collect();
            let tail_time = len as u64 * 60_000;
            let mut chart = CandlestickChart::new(1);
            chart.set_secondary_series_identity("SYNTH".into(), "Synthetic".into());
            chart.candles = initial.clone();
            chart.secondary_series.as_mut().expect("secondary").candles = initial.clone();

            assert_eq!(
                push(&mut chart, secondary, initial[0].clone()),
                CandlePushResult::RejectedOutOfOrder
            );
            assert_eq!(
                candle_bits(series(&chart, secondary)),
                candle_bits(&initial)
            );
            let invalid = Candle {
                volume: -1.0,
                ..Candle::test_price(tail_time + 60_000, 20.0)
            };
            assert_eq!(
                push(&mut chart, secondary, invalid),
                CandlePushResult::RejectedInvalid
            );
            assert_eq!(
                candle_bits(series(&chart, secondary)),
                candle_bits(&initial)
            );

            let replacement = Candle::test_price(tail_time, 20_000.0);
            assert_eq!(
                push(&mut chart, secondary, replacement.clone()),
                CandlePushResult::Updated
            );
            let mut expected = initial.clone();
            expected[len - 1] = replacement;
            assert_eq!(
                candle_bits(series(&chart, secondary)),
                candle_bits(&expected)
            );

            let next = Candle::test_price(tail_time + 60_000, 30_000.0);
            assert_eq!(
                push(&mut chart, secondary, next.clone()),
                CandlePushResult::Appended
            );
            expected.push(next);
            assert_eq!(
                candle_bits(series(&chart, secondary)),
                candle_bits(&expected[len + 1 - MAX_CHART_CANDLES..])
            );
            assert_eq!(
                candle_bits(series(&chart, !secondary)),
                candle_bits(&initial)
            );
            assert!(matches!(chart.status, ChartStatus::Loading));
        }
    }
}

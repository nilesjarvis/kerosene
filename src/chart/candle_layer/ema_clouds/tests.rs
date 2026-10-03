use super::*;

#[test]
fn ema_cloud_series_aligns_different_warmup_periods() {
    let candles: Vec<_> = [10.0, 20.0, 50.0, 30.0]
        .into_iter()
        .enumerate()
        .map(|(i, price)| Candle::test_flat(i as u64 * 1_000, price))
        .collect();
    let series = ema_cloud_series(&candles, 2, 3);
    assert_eq!(series.len(), 2);
    assert_eq!(series[0].0, 2_000);
    assert!((series[0].1 - 115.0 / 3.0).abs() < 1e-10);
    assert!((series[0].2 - 80.0 / 3.0).abs() < 1e-10);
    let reversed = ema_cloud_series(&candles, 3, 2);
    assert_eq!(reversed[0], (series[0].0, series[0].2, series[0].1));
    assert!(ema_cloud_series(&candles, 2, 5).is_empty());
    assert!(ema_cloud_series(&candles, 0, 2).is_empty());
    assert!(ema_cloud_series(&[], 2, 3).is_empty());
}

#[test]
fn ema_cloud_sampling_respects_source_timestamps_and_visible_range() {
    let candles: Vec<_> = (0..10)
        .map(|i| Candle::test_flat(i * 1_000, 10.0))
        .collect();
    let series = [(2_000, 20.0, 10.0), (5_000, 40.0, 30.0)];
    assert_eq!(
        visible_cloud_samples(&candles, &series, 3, 5),
        vec![
            (2, 20.0, 10.0),
            (3, 20.0, 10.0),
            (4, 20.0, 10.0),
            (5, 40.0, 30.0),
            (6, 40.0, 30.0),
        ]
    );
    assert!(visible_cloud_samples(&candles, &series, 0, 0).is_empty());
    assert!(visible_cloud_samples(&candles, &series, 10, 20).is_empty());
}

#[test]
fn ema_cloud_crossings_split_at_the_intersection_on_both_axis_directions() {
    for scale in [1.0, -1.0] {
        let bands = cloud_bands(&[
            CloudEdge {
                fast: Point::new(0.0, 10.0 * scale),
                slow: Point::new(0.0, 20.0 * scale),
                bullish: true,
            },
            CloudEdge {
                fast: Point::new(10.0, 30.0 * scale),
                slow: Point::new(10.0, 20.0 * scale),
                bullish: false,
            },
        ]);
        assert_eq!(bands.len(), 2);
        assert!(bands[0].bullish);
        assert!(!bands[1].bullish);
        let crossing = Point::new(5.0, 20.0 * scale);
        assert_eq!(bands[0].points[1], crossing);
        assert_eq!(bands[1].points[0], crossing);
        assert!(bands[0].points.iter().all(|point| point.x <= 5.0));
        assert!(bands[1].points.iter().all(|point| point.x >= 5.0));
    }
}

#[test]
fn ema_cloud_contiguous_runs_have_no_per_candle_seams_and_skip_invalid_points() {
    let edges: Vec<_> = (0..4)
        .map(|i| CloudEdge {
            fast: Point::new(i as f32, 10.0),
            slow: Point::new(i as f32, 20.0),
            bullish: true,
        })
        .collect();
    let bands = cloud_bands(&edges);
    assert_eq!(bands.len(), 1);
    assert_eq!(bands[0].points.len(), 8);
    let mut broken = edges;
    broken[2].fast.y = f32::NAN;
    let bands = cloud_bands(&broken);
    assert_eq!(bands.len(), 1);
    assert_eq!(bands[0].points.len(), 4);
    assert!(cloud_bands(&[]).is_empty());
}

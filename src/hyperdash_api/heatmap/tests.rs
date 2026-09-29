use super::parsing::{HYPERDASH_HEATMAP_MAX_CELLS, cap_heatmap_rects, parse_heatmap_response};
use super::{
    HYPERDASH_HEATMAP_DEFAULT_BUCKET_SECS, HYPERDASH_HEATMAP_MAX_LOOKBACK_SECS,
    infer_heatmap_bucket_duration_ms, normalize_heatmap_time_range, parse_heatmap_timestamp,
};
use crate::hyperdash_api::{HeatmapFetchParams, HeatmapRect};

#[test]
fn heatmap_timestamp_parser_uses_utc_epoch_millis() {
    assert_eq!(
        parse_heatmap_timestamp("2026-05-01 13:00:00"),
        Some(1_777_640_400_000)
    );
}

#[test]
fn heatmap_bucket_duration_infers_smallest_positive_gap() {
    assert_eq!(
        infer_heatmap_bucket_duration_ms(&mut [
            1_777_640_400_000,
            1_777_647_600_000,
            1_777_644_000_000,
        ]),
        HYPERDASH_HEATMAP_DEFAULT_BUCKET_SECS * 1000
    );
}

#[test]
fn heatmap_bucket_duration_handles_duplicates_empty_and_extreme_timestamps() {
    let default_ms = HYPERDASH_HEATMAP_DEFAULT_BUCKET_SECS * 1000;
    for (mut timestamps, expected) in [
        (vec![], default_ms),
        (vec![7], default_ms),
        (vec![7, 7, 7], default_ms),
        (vec![900, 100, 300, 100], 200),
        (vec![u64::MAX, 0], u64::MAX),
        (vec![u64::MAX, u64::MAX - 1, 0], 1),
        (vec![8_000_000, 0, 4_000_000], 4_000_000),
    ] {
        assert_eq!(infer_heatmap_bucket_duration_ms(&mut timestamps), expected);
    }
}

#[test]
fn heatmap_duration_includes_timestamps_from_discarded_cells() {
    let body = serde_json::json!({
        "data": { "analytics": { "liquidationLevels": { "bands": [
            {
                "minPrice": 10.0, "maxPrice": 20.0,
                "historicalData": [
                    { "timestamp": "2026-05-01 13:00:00", "totalAmount": 2.0 },
                    { "timestamp": "2026-05-01 14:00:00", "totalAmount": -3.0 },
                    { "timestamp": "2026-05-01 13:00:00", "totalAmount": 0.0 },
                    { "timestamp": "invalid", "totalAmount": 9.0 }
                ]
            },
            {
                "minPrice": 1e308, "maxPrice": 1e308,
                "historicalData": [
                    { "timestamp": "2026-05-01 13:15:00", "totalAmount": 1.0 }
                ]
            }
        ] } } },
        "errors": [{ "message": "partial warning" }]
    });

    let heatmap = parse_heatmap_response(&body.to_string()).expect("fixture heatmap");

    assert_eq!(heatmap.max_abs_usd, 45.0);
    let cells: Vec<_> = heatmap
        .rects
        .iter()
        .map(|cell| {
            (
                cell.timestamp_ms,
                cell.duration_ms,
                cell.price_lo,
                cell.price_hi,
                cell.amount_coins,
                cell.amount_usd,
            )
        })
        .collect();
    assert_eq!(
        cells,
        vec![
            (1_777_640_400_000, 900_000, 10.0, 20.0, 2.0, 30.0),
            (1_777_644_000_000, 900_000, 10.0, 20.0, -3.0, -45.0),
            (1_777_640_400_000, 900_000, 10.0, 20.0, 0.0, 0.0),
        ]
    );
}

#[test]
fn heatmap_cell_cap_keeps_high_notional_cells_in_time_order() {
    let mut rects: Vec<_> = (0..HYPERDASH_HEATMAP_MAX_CELLS + 10)
        .rev()
        .map(|idx| HeatmapRect {
            timestamp_ms: idx as u64,
            duration_ms: 3_600_000,
            price_lo: idx as f64,
            price_hi: idx as f64 + 1.0,
            amount_coins: idx as f64,
            amount_usd: idx as f64,
        })
        .collect();
    rects[0].amount_usd = 2_000_000.0;
    rects[1].amount_usd = -1_000_000.0;

    cap_heatmap_rects(&mut rects);

    assert_eq!(rects.len(), HYPERDASH_HEATMAP_MAX_CELLS);
    assert!(rects.iter().any(|rect| rect.amount_usd == 2_000_000.0));
    assert!(rects.iter().any(|rect| rect.amount_usd == -1_000_000.0));
    assert!(!rects.iter().any(|rect| rect.amount_usd == 0.0));
    assert!(
        rects
            .windows(2)
            .all(|pair| pair[0].timestamp_ms <= pair[1].timestamp_ms)
    );
}

#[test]
fn heatmap_time_range_caps_to_recent_api_window() {
    let now = 2_000_000;
    let start = now - HYPERDASH_HEATMAP_MAX_LOOKBACK_SECS * 2;
    assert_eq!(
        normalize_heatmap_time_range(start, now, now),
        Some((now - HYPERDASH_HEATMAP_MAX_LOOKBACK_SECS, now))
    );
}

#[test]
fn heatmap_time_range_expands_short_windows_to_one_bucket() {
    let now = 2_000_000;
    assert_eq!(
        normalize_heatmap_time_range(now - 300, now, now),
        Some((now - HYPERDASH_HEATMAP_DEFAULT_BUCKET_SECS, now))
    );
}

#[test]
fn heatmap_time_range_rejects_ranges_older_than_api_window() {
    let now = 2_000_000;
    let old_end = now - HYPERDASH_HEATMAP_MAX_LOOKBACK_SECS - 1;
    assert_eq!(
        normalize_heatmap_time_range(old_end - 3600, old_end, now),
        None
    );
}

#[test]
fn heatmap_fetch_params_refresh_after_new_hourly_bucket() {
    let prev = HeatmapFetchParams {
        coin: "BTC".to_string(),
        min_price: 70_000.0,
        max_price: 90_000.0,
        start_time: 1_000_000,
        end_time: 1_003_600,
    };

    assert!(!prev.needs_refetch("BTC", 70_000.0, 90_000.0, 1_000_060, 1_003_660,));
    assert!(prev.needs_refetch("BTC", 70_000.0, 90_000.0, 1_003_600, 1_007_200,));
}

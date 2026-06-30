use super::{
    HYPERDASH_HEATMAP_DEFAULT_BUCKET_SECS, HYPERDASH_HEATMAP_MAX_CELLS,
    HYPERDASH_HEATMAP_MAX_LOOKBACK_SECS, cap_heatmap_rects, infer_heatmap_bucket_duration_ms,
    normalize_heatmap_time_range, parse_heatmap_timestamp,
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
        infer_heatmap_bucket_duration_ms(&[
            1_777_640_400_000,
            1_777_647_600_000,
            1_777_644_000_000,
        ]),
        HYPERDASH_HEATMAP_DEFAULT_BUCKET_SECS * 1000
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

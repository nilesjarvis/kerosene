use crate::api::Candle;
use crate::chart::ChartViewport;
use crate::hyperdash_api::HeatmapFetchParams;

use super::*;

fn candle(open_time: u64, low: f64, high: f64) -> Candle {
    Candle {
        open_time,
        close_time: open_time + 59_999,
        open: low,
        high,
        low,
        close: high,
        volume: 10.0,
    }
}

fn context<'a>(
    candles: &'a [Candle],
    previous: Option<&'a HeatmapFetchParams>,
) -> HeatmapRequestContext<'a> {
    HeatmapRequestContext {
        show_heatmap: true,
        symbol: "BTC",
        heatmap_fetching: false,
        muted: false,
        coin: Some("BTC".to_string()),
        candles,
        viewport: None,
        previous,
        now_time: 10_000,
    }
}

fn assert_near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn request_planner_skips_disabled_muted_fetching_or_unsupported_inputs() {
    let candles = vec![candle(1_000, 90.0, 110.0), candle(2_000, 95.0, 120.0)];

    let mut disabled = context(&candles, None);
    disabled.show_heatmap = false;
    assert!(plan_heatmap_fetch_request(disabled).unwrap().is_none());

    let mut fetching = context(&candles, None);
    fetching.heatmap_fetching = true;
    assert!(plan_heatmap_fetch_request(fetching).unwrap().is_none());

    let mut muted = context(&candles, None);
    muted.muted = true;
    assert!(plan_heatmap_fetch_request(muted).unwrap().is_none());

    let mut unsupported = context(&candles, None);
    unsupported.coin = None;
    assert!(plan_heatmap_fetch_request(unsupported).unwrap().is_none());

    let empty: Vec<Candle> = Vec::new();
    assert!(
        plan_heatmap_fetch_request(context(&empty, None))
            .unwrap()
            .is_none()
    );
}

#[test]
fn request_planner_builds_candle_derived_range() {
    let candles = vec![candle(1_000, 90.0, 110.0), candle(2_000, 95.0, 120.0)];
    let request = plan_heatmap_fetch_request(context(&candles, None))
        .unwrap()
        .unwrap();

    assert_eq!(request.coin, "BTC");
    assert_eq!(request.start_time, 0);
    assert_eq!(request.end_time, 2);
    assert_near(request.min_price, 88.5);
    assert_near(request.max_price, 121.5);
}

#[test]
fn request_planner_caps_very_large_time_ranges() {
    let now = 2_000_000;
    let candles = vec![
        candle((now - 7 * 24 * 60 * 60) * 1000, 90.0, 110.0),
        candle(now * 1000, 95.0, 120.0),
    ];
    let mut ctx = context(&candles, None);
    ctx.now_time = now;

    let request = plan_heatmap_fetch_request(ctx).unwrap().unwrap();

    assert_eq!(request.start_time, now - HEATMAP_MAX_REQUEST_SPAN_SECS);
    assert_eq!(request.end_time, now);
}

#[test]
fn request_planner_uses_viewport_price_range_when_available() {
    let candles = vec![candle(1_000, 90.0, 110.0), candle(2_000, 95.0, 120.0)];
    let mut ctx = context(&candles, None);
    ctx.viewport = Some(ChartViewport {
        start_time_ms: 1_000,
        end_time_ms: 2_000,
        price_lo: 50.0,
        price_hi: 150.0,
        chart_width: 0.0,
        candle_width: 0.0,
        scroll_offset: 0.0,
        y_auto: true,
        y_scale: 1.0,
        y_offset: 0.0,
        funding_y_scale: 1.0,
        funding_y_offset: 0.0,
    });

    let request = plan_heatmap_fetch_request(ctx).unwrap().unwrap();

    assert_near(request.min_price, 45.0);
    assert_near(request.max_price, 155.0);
}

#[test]
fn request_planner_skips_when_previous_fetch_still_covers_range() {
    let candles = vec![candle(1_000, 90.0, 110.0), candle(2_000, 95.0, 120.0)];
    let previous = HeatmapFetchParams {
        coin: "BTC".to_string(),
        min_price: 88.5,
        max_price: 121.5,
        start_time: 0,
        end_time: 2,
    };

    let request = plan_heatmap_fetch_request(context(&candles, Some(&previous))).unwrap();

    assert!(request.is_none());
}

#[test]
fn request_planner_reports_out_of_range_history() {
    let candles = vec![candle(1_000, 90.0, 110.0), candle(2_000, 95.0, 120.0)];
    let mut ctx = context(&candles, None);
    ctx.now_time = 0;

    let error = plan_heatmap_fetch_request(ctx).unwrap_err();

    assert_eq!(error, "HEAT only has recent HyperDash history");
}

use crate::api::Candle;
use crate::chart::ChartViewport;
use crate::hyperdash_api::HeatmapFetchParams;

use super::*;

mod errors;
mod previous;
mod range;
mod skips;
mod viewport;

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

fn request_or_panic(ctx: HeatmapRequestContext<'_>) -> HeatmapFetchParams {
    match plan_heatmap_fetch_request(ctx) {
        Ok(Some(request)) => request,
        Ok(None) => panic!("expected heatmap request"),
        Err(error) => panic!("expected heatmap request, got error: {error}"),
    }
}

fn optional_request_or_panic(ctx: HeatmapRequestContext<'_>) -> Option<HeatmapFetchParams> {
    match plan_heatmap_fetch_request(ctx) {
        Ok(request) => request,
        Err(error) => panic!("expected optional heatmap request, got error: {error}"),
    }
}

fn error_or_panic(ctx: HeatmapRequestContext<'_>) -> String {
    match plan_heatmap_fetch_request(ctx) {
        Ok(_) => panic!("expected heatmap request planning error"),
        Err(error) => error,
    }
}

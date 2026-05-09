use crate::api::Candle;
use crate::chart::ChartViewport;
use crate::hyperdash_api::{HeatmapFetchParams, normalize_heatmap_time_range};

use super::range::heatmap_price_range_for_request;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Heatmap Request Planning
// ---------------------------------------------------------------------------

const HEATMAP_MAX_REQUEST_SPAN_SECS: u64 = 72 * 60 * 60;

pub(super) struct HeatmapRequestContext<'a> {
    pub(super) show_heatmap: bool,
    pub(super) symbol: &'a str,
    pub(super) heatmap_fetching: bool,
    pub(super) muted: bool,
    pub(super) coin: Option<String>,
    pub(super) candles: &'a [Candle],
    pub(super) viewport: Option<ChartViewport>,
    pub(super) previous: Option<&'a HeatmapFetchParams>,
    pub(super) now_time: u64,
}

fn cap_heatmap_request_time_span(start_time: u64, end_time: u64) -> (u64, u64) {
    (
        start_time.max(end_time.saturating_sub(HEATMAP_MAX_REQUEST_SPAN_SECS)),
        end_time,
    )
}

pub(super) fn plan_heatmap_fetch_request(
    ctx: HeatmapRequestContext<'_>,
) -> Result<Option<HeatmapFetchParams>, String> {
    if !ctx.show_heatmap || ctx.symbol.is_empty() || ctx.heatmap_fetching || ctx.muted {
        return Ok(None);
    }

    let Some(coin) = ctx.coin else {
        return Ok(None);
    };
    if ctx.candles.is_empty() {
        return Ok(None);
    }

    let candidate_start = ctx
        .viewport
        .map(|viewport| viewport.start_time_ms / 1000)
        .or_else(|| ctx.candles.first().map(|candle| candle.open_time / 1000))
        .unwrap_or(0);
    let candidate_end = ctx
        .viewport
        .map(|viewport| viewport.end_time_ms / 1000)
        .or_else(|| ctx.candles.last().map(|candle| candle.open_time / 1000))
        .unwrap_or(0);

    let Some((start_time, end_time)) =
        normalize_heatmap_time_range(candidate_start, candidate_end, ctx.now_time)
    else {
        return Err("HEAT only has recent HyperDash history".to_string());
    };
    let (start_time, end_time) = cap_heatmap_request_time_span(start_time, end_time);

    let Some((min_price, max_price)) =
        heatmap_price_range_for_request(ctx.candles, start_time, end_time, ctx.viewport)
    else {
        return Ok(None);
    };

    if let Some(prev) = ctx.previous
        && !prev.needs_refetch(&coin, min_price, max_price, start_time, end_time)
    {
        return Ok(None);
    }

    Ok(Some(HeatmapFetchParams {
        coin,
        min_price,
        max_price,
        start_time,
        end_time,
    }))
}

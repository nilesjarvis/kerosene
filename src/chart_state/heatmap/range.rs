use crate::api::Candle;
use crate::chart::ChartViewport;

// ---------------------------------------------------------------------------
// Heatmap Request Ranges
// ---------------------------------------------------------------------------

pub(super) fn padded_heatmap_price_range(price_lo: f64, price_hi: f64) -> Option<(f64, f64)> {
    if !price_lo.is_finite() || !price_hi.is_finite() || price_hi <= price_lo {
        return None;
    }

    let range = price_hi - price_lo;
    let reference = ((price_hi + price_lo) * 0.5).abs().max(1.0);
    let pad = (range * 0.05).max(reference * 0.0025);
    Some(((price_lo - pad).max(0.0), price_hi + pad))
}

pub(super) fn heatmap_price_range_for_request(
    candles: &[Candle],
    start_time: u64,
    end_time: u64,
    viewport: Option<ChartViewport>,
) -> Option<(f64, f64)> {
    if let Some(viewport) = viewport
        && let Some(range) = padded_heatmap_price_range(viewport.price_lo, viewport.price_hi)
    {
        return Some(range);
    }

    let start_ms = start_time.saturating_mul(1000);
    let end_ms = end_time.saturating_mul(1000);
    let mut price_lo = f64::INFINITY;
    let mut price_hi = f64::NEG_INFINITY;

    for candle in candles {
        if candle.open_time < start_ms || candle.open_time > end_ms {
            continue;
        }
        price_lo = price_lo.min(candle.low);
        price_hi = price_hi.max(candle.high);
    }

    if price_hi <= price_lo {
        for candle in candles {
            price_lo = price_lo.min(candle.low);
            price_hi = price_hi.max(candle.high);
        }
    }

    padded_heatmap_price_range(price_lo, price_hi)
}

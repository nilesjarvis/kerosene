use super::PRICE_PADDING_PCT;
use crate::api::Candle;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::chart) struct VisiblePriceStats {
    pub(in crate::chart) price_lo: f64,
    pub(in crate::chart) price_hi: f64,
    pub(in crate::chart) price_range: f64,
    pub(in crate::chart) volume_max: f64,
}

pub(in crate::chart) fn visible_price_stats(
    candles: &[Candle],
    y_auto: bool,
    y_scale: f64,
    y_offset: f64,
) -> Option<VisiblePriceStats> {
    if candles.is_empty() {
        return None;
    }

    let (auto_lo, auto_hi, volume_max) = candles.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY, 0.0_f64),
        |(lo, hi, volume_max), candle| {
            (
                lo.min(candle.low),
                hi.max(candle.high),
                volume_max.max(candle.volume),
            )
        },
    );
    let auto_pad = (auto_hi - auto_lo) * PRICE_PADDING_PCT;
    let auto_lo = auto_lo - auto_pad;
    let auto_hi = auto_hi + auto_pad;

    let (price_lo, price_hi) = if y_auto {
        (auto_lo, auto_hi)
    } else {
        let auto_range = auto_hi - auto_lo;
        let auto_mid = (auto_hi + auto_lo) * 0.5;
        let scaled_range = auto_range * y_scale;
        let mid = auto_mid + y_offset;
        (mid - scaled_range * 0.5, mid + scaled_range * 0.5)
    };

    Some(VisiblePriceStats {
        price_lo,
        price_hi,
        price_range: price_hi - price_lo,
        volume_max,
    })
}

pub(in crate::chart) fn visible_price_stats_with_follow(
    candles: &[Candle],
    y_auto: bool,
    y_scale: f64,
    y_offset: f64,
    follow_price: bool,
    follow_center: Option<f64>,
) -> Option<VisiblePriceStats> {
    if !follow_price {
        return visible_price_stats(candles, y_auto, y_scale, y_offset);
    }

    let auto_stats = visible_price_stats(candles, true, 1.0, 0.0)?;
    let Some(center) = follow_center.filter(|value| value.is_finite()) else {
        return visible_price_stats(candles, y_auto, y_scale, y_offset);
    };

    let price_range = if y_auto {
        auto_stats.price_range
    } else {
        auto_stats.price_range * y_scale
    };
    if price_range <= 0.0 || !price_range.is_finite() {
        return Some(auto_stats);
    }

    let half_range = price_range * 0.5;
    Some(VisiblePriceStats {
        price_lo: center - half_range,
        price_hi: center + half_range,
        price_range,
        volume_max: auto_stats.volume_max,
    })
}

use crate::helpers::{nice_step_ceil, trim_decimal_zeros};

use iced::Point;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Depth Chart Geometry
// ---------------------------------------------------------------------------

/// Fraction of the canvas height left clear above the tallest depth step so
/// the curve never touches the top edge.
const TOP_MARGIN_FRACTION: f32 = 0.10;

/// Price/size scale for one depth chart paint: a symmetric price window
/// around the mid mapped to the canvas width, and a quantized cumulative-size
/// maximum mapped to the canvas height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct DepthChartLayout {
    pub mid: f64,
    pub half_range: f64,
    pub max_cum: f64,
    pub tick: f64,
    pub width: f32,
    pub height: f32,
}

impl DepthChartLayout {
    pub fn price_min(&self) -> f64 {
        self.mid - self.half_range
    }

    pub fn price_max(&self) -> f64 {
        self.mid + self.half_range
    }

    pub fn x_for_price(&self, price: f64) -> f32 {
        let t = (price - self.price_min()) / (self.half_range * 2.0);
        t as f32 * self.width
    }

    pub fn price_at_x(&self, x: f32) -> f64 {
        self.price_min() + (x / self.width) as f64 * self.half_range * 2.0
    }

    pub fn y_for_cum(&self, cum: f64) -> f32 {
        let t = ((cum / self.max_cum) as f32).clamp(0.0, 1.0);
        self.height - t * self.height * (1.0 - TOP_MARGIN_FRACTION)
    }
}

/// Build the paint scale for the given aggregated sides. The price window is
/// the smaller side's coverage quantized to a 1-2-5 number of ticks, so
/// screen positions only change when depth coverage crosses a quantization
/// boundary instead of on every book update.
pub(super) fn depth_chart_layout(
    bids: &[(f64, f64, f64)],
    asks: &[(f64, f64, f64)],
    mid: f64,
    tick: f64,
    width: f32,
    height: f32,
) -> Option<DepthChartLayout> {
    if !(mid.is_finite() && mid > 0.0 && tick.is_finite() && tick > 0.0) {
        return None;
    }
    if !(width > 0.0 && height > 0.0) {
        return None;
    }

    let bid_extent = bids
        .last()
        .map(|&(px, _, _)| mid - px)
        .filter(|extent| *extent > 0.0);
    let ask_extent = asks
        .last()
        .map(|&(px, _, _)| px - mid)
        .filter(|extent| *extent > 0.0);
    let raw_half = match (bid_extent, ask_extent) {
        (Some(bid), Some(ask)) => bid.min(ask),
        (Some(bid), None) => bid,
        (None, Some(ask)) => ask,
        (None, None) => return None,
    };
    // Aggregation pushes bucket prices outward from the raw mid by up to one
    // tick, so a window sized by the smaller side's extent could cut the
    // other side's best bucket; widen so every best bucket stays visible.
    let raw_half = bids
        .first()
        .map(|&(px, _, _)| mid - px)
        .into_iter()
        .chain(asks.first().map(|&(px, _, _)| px - mid))
        .fold(raw_half, f64::max);
    // Quantization can inflate the window past the mid itself on thin
    // low-priced markets; cap it so the axis never reaches below zero.
    let half_range = (nice_step_ceil((raw_half / tick).max(1.0)) * tick).min(mid);

    let mut layout = DepthChartLayout {
        mid,
        half_range,
        max_cum: 1.0,
        tick,
        width,
        height,
    };
    let max_in_range = side_max_cum(bids, &layout).max(side_max_cum(asks, &layout));
    if max_in_range <= 0.0 {
        return None;
    }
    layout.max_cum = nice_step_ceil_fractional(max_in_range);

    Some(layout)
}

/// 1-2-5 ceil that extends below 1.0. The shared `nice_step_ceil` floors at
/// 1.0, which suits bar normalization but would squash a book whose
/// in-window cumulative depth is under one base unit (thin high-unit-price
/// markets) into the bottom sliver of the canvas.
fn nice_step_ceil_fractional(value: f64) -> f64 {
    if !value.is_finite() || value <= 0.0 {
        return 1.0;
    }
    if value >= 1.0 {
        return nice_step_ceil(value);
    }
    let exponent = value.log10().floor();
    let base = 10f64.powf(exponent);
    let mantissa = value / base;
    let stepped = if mantissa <= 1.0 {
        1.0
    } else if mantissa <= 2.0 {
        2.0
    } else if mantissa <= 5.0 {
        5.0
    } else {
        10.0
    };
    stepped * base
}

/// Largest cumulative size a side reaches inside the price window. Levels are
/// ordered best-first, so the walk stops at the first level past the window.
fn side_max_cum(levels: &[(f64, f64, f64)], layout: &DepthChartLayout) -> f64 {
    let mut max_cum = 0.0;
    for &(px, _, cum) in levels {
        if px < layout.price_min() || px > layout.price_max() {
            break;
        }
        max_cum = cum;
    }
    max_cum
}

/// Screen-space polyline for one side's cumulative step curve, from the best
/// level at the baseline outward, ending flat at the chart edge. Levels
/// outside the price window are not walked.
pub(super) fn side_points(
    levels: &[(f64, f64, f64)],
    layout: &DepthChartLayout,
    is_bid: bool,
) -> Vec<Point> {
    let mut points = Vec::with_capacity(levels.len() * 2 + 1);
    let mut last_cum = 0.0;

    for &(px, _, cum) in levels {
        if px < layout.price_min() || px > layout.price_max() {
            break;
        }
        let x = layout.x_for_price(px);
        points.push(Point::new(x, layout.y_for_cum(last_cum)));
        points.push(Point::new(x, layout.y_for_cum(cum)));
        last_cum = cum;
    }

    if let Some(last) = points.last().copied() {
        let edge_x = if is_bid { 0.0 } else { layout.width };
        if (last.x - edge_x).abs() > f32::EPSILON {
            points.push(Point::new(edge_x, last.y));
        }
    }

    points
}

/// Cumulative size resting at `price` or better: for bids, every level at or
/// above `price`; for asks, every level at or below. `None` when `price` is
/// ahead of the side's best level.
pub(super) fn cum_at_price(levels: &[(f64, f64, f64)], price: f64, is_bid: bool) -> Option<f64> {
    let mut depth = None;
    for &(px, _, cum) in levels {
        let covered = if is_bid { px >= price } else { px <= price };
        if !covered {
            break;
        }
        depth = Some(cum);
    }
    depth
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HoverTarget {
    pub price: f64,
    pub cum: f64,
    pub is_bid: bool,
}

/// Resolve a cursor x position to the tick bucket under it and the cumulative
/// depth at that bucket. The cursor price is bucketed inward (toward the
/// mid): levels sit on the tick grid, so the inward bucket's cumulative is
/// exactly the painted step height under the cursor.
pub(super) fn hover_target(
    bids: &[(f64, f64, f64)],
    asks: &[(f64, f64, f64)],
    layout: &DepthChartLayout,
    x: f32,
) -> Option<HoverTarget> {
    let raw_price = layout.price_at_x(x);
    if !raw_price.is_finite() {
        return None;
    }
    let is_bid = raw_price < layout.mid;
    let (levels, price) = if is_bid {
        (bids, (raw_price / layout.tick).ceil() * layout.tick)
    } else {
        (asks, (raw_price / layout.tick).floor() * layout.tick)
    };
    if price <= 0.0 {
        return None;
    }
    let cum = cum_at_price(levels, price, is_bid)?;
    Some(HoverTarget { price, cum, is_bid })
}

/// Screen x positions of the marker prices that fall inside the price window.
pub(super) fn marker_xs(prices: &[f64], layout: &DepthChartLayout) -> Vec<f32> {
    prices
        .iter()
        .filter(|px| **px >= layout.price_min() && **px <= layout.price_max())
        .map(|&px| layout.x_for_price(px))
        .collect()
}

/// Compact label for the cumulative-size axis: 1-2-5 quantized maxima render
/// as values like "500", "1.25K", or "2.5M".
pub(super) fn axis_size_label(value: f64) -> String {
    if !value.is_finite() || value <= 0.0 {
        return "0".to_string();
    }
    if value >= 1_000_000.0 {
        format!(
            "{}M",
            trim_decimal_zeros(format!("{:.2}", value / 1_000_000.0))
        )
    } else if value >= 1_000.0 {
        format!("{}K", trim_decimal_zeros(format!("{:.2}", value / 1_000.0)))
    } else if value >= 1.0 {
        trim_decimal_zeros(format!("{value:.2}"))
    } else {
        // Sub-unit scales (thin high-unit-price books) need the extra
        // precision: a 0.05 maximum quarters into 0.0125 steps.
        trim_decimal_zeros(format!("{value:.4}"))
    }
}

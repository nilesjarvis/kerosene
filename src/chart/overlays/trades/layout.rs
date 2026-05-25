use super::TradingOverlayContext;
use super::grouping::TradeMarkerGroup;
use crate::api::Candle;

use iced::Point;

// ---------------------------------------------------------------------------
// Trade Marker Layout
// ---------------------------------------------------------------------------

pub(super) const TRADE_MARKER_RADIUS: f32 = 3.2;
const TRADE_MARKER_EDGE_PADDING: f32 = 1.0;
pub(super) const TRADE_MARKER_MIN_PRICE_HEIGHT: f32 =
    (TRADE_MARKER_RADIUS + TRADE_MARKER_EDGE_PADDING) * 2.0;
pub(super) const TRADE_MARKER_CANDLE_GAP: f32 = 8.0;
const TRADE_MARKER_STACK_GAP: f32 = 7.0;
const TRADE_MARKER_MAX_STACK: usize = 4;
const TRADE_MARKER_HOVER_DISTANCE: f32 = 8.0;

#[derive(Debug, Clone, Copy)]
pub(super) struct TradeMarkerGroupLayout {
    dots: [TradeMarkerDot; TRADE_MARKER_MAX_STACK],
    dot_count: usize,
}

impl TradeMarkerGroupLayout {
    pub(super) fn visible_dots(&self) -> &[TradeMarkerDot] {
        &self.dots[..self.dot_count]
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct TradeMarkerDot {
    pub(super) center: Point,
    pub(super) radius: f32,
}

pub(super) fn trade_marker_layout<PriceToY, IdxToCx>(
    ctx: &TradingOverlayContext<'_, PriceToY, IdxToCx>,
    group: &TradeMarkerGroup,
) -> Option<TradeMarkerGroupLayout>
where
    PriceToY: Fn(f64) -> f32,
    IdxToCx: Fn(usize) -> f32,
{
    let x = (ctx.idx_to_cx)(group.candle_idx);
    if x < -TRADE_MARKER_RADIUS || x > ctx.chart_w + TRADE_MARKER_RADIUS {
        return None;
    }

    let (marker_min_y, marker_max_y) = trade_marker_clamp_bounds(ctx.price_h)?;

    let candle = ctx.candles.get(group.candle_idx)?;
    let base_y = trade_marker_anchor_y(candle, group.is_buy, ctx.price_to_y)?;

    let dot_count = group.count.min(TRADE_MARKER_MAX_STACK);
    if dot_count == 0 {
        return None;
    }
    let stack_direction = if group.is_buy { 1.0 } else { -1.0 };
    let max_stack_offset = (dot_count.saturating_sub(1) as f32) * TRADE_MARKER_STACK_GAP;
    let stack_min_y = if group.is_buy {
        base_y
    } else {
        base_y - max_stack_offset
    };
    let stack_max_y = if group.is_buy {
        base_y + max_stack_offset
    } else {
        base_y
    };
    let edge_tolerance = TRADE_MARKER_CANDLE_GAP + TRADE_MARKER_RADIUS;
    if stack_max_y < -edge_tolerance || stack_min_y > ctx.price_h + edge_tolerance {
        return None;
    }

    let mut dots = [TradeMarkerDot {
        center: Point::new(0.0, 0.0),
        radius: TRADE_MARKER_RADIUS,
    }; TRADE_MARKER_MAX_STACK];
    for (stack_idx, dot) in dots.iter_mut().enumerate().take(dot_count) {
        let y = (base_y + stack_direction * stack_idx as f32 * TRADE_MARKER_STACK_GAP)
            .clamp(marker_min_y, marker_max_y);
        *dot = TradeMarkerDot {
            center: Point::new(x, y),
            radius: trade_marker_dot_radius(group.count, dot_count, stack_idx),
        };
    }

    Some(TradeMarkerGroupLayout { dots, dot_count })
}

pub(super) fn trade_marker_clamp_bounds(price_h: f32) -> Option<(f32, f32)> {
    (price_h >= TRADE_MARKER_MIN_PRICE_HEIGHT).then_some((
        TRADE_MARKER_RADIUS + TRADE_MARKER_EDGE_PADDING,
        price_h - TRADE_MARKER_RADIUS - TRADE_MARKER_EDGE_PADDING,
    ))
}

pub(super) fn trade_marker_dot_radius(
    _group_count: usize,
    _dot_count: usize,
    _stack_idx: usize,
) -> f32 {
    TRADE_MARKER_RADIUS
}

pub(super) fn trade_marker_anchor_y<PriceToY>(
    candle: &Candle,
    is_buy: bool,
    price_to_y: &PriceToY,
) -> Option<f32>
where
    PriceToY: Fn(f64) -> f32,
{
    let high_y = price_to_y(candle.high);
    let low_y = price_to_y(candle.low);
    if !high_y.is_finite() || !low_y.is_finite() {
        return None;
    }

    let visual_top = high_y.min(low_y);
    let visual_bottom = high_y.max(low_y);
    let visual_gap = TRADE_MARKER_CANDLE_GAP + TRADE_MARKER_RADIUS;
    Some(if is_buy {
        visual_bottom + visual_gap
    } else {
        visual_top - visual_gap
    })
}

pub(super) fn nearest_trade_marker_group<'a, PriceToY, IdxToCx>(
    ctx: &TradingOverlayContext<'_, PriceToY, IdxToCx>,
    groups: &'a [TradeMarkerGroup],
    pos: Point,
) -> Option<(&'a TradeMarkerGroup, f32)>
where
    PriceToY: Fn(f64) -> f32,
    IdxToCx: Fn(usize) -> f32,
{
    let mut best: Option<(&TradeMarkerGroup, f32)> = None;
    for group in groups {
        let Some(layout) = trade_marker_layout(ctx, group) else {
            continue;
        };
        for dot in layout.visible_dots() {
            let dx = pos.x - dot.center.x;
            let dy = pos.y - dot.center.y;
            let dist = (dx * dx + dy * dy).sqrt();
            let hit_distance = dot.radius + TRADE_MARKER_HOVER_DISTANCE;
            if dist <= hit_distance && best.is_none_or(|(_, best_dist)| dist < best_dist) {
                best = Some((group, dist));
            }
        }
    }
    best
}

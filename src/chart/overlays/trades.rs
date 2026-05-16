use super::TradingOverlayContext;
use crate::api::Candle;
use crate::chart::model::{CandlestickChart, TradeMarker};
use crate::chart::tooltips::{TooltipLine, TooltipSurface};
use crate::helpers::{format_price, format_timestamp_exact};
use iced::widget::canvas;
use iced::{Color, Point, Size};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Trade Marker Overlays
// ---------------------------------------------------------------------------

const TRADE_MARKER_RADIUS: f32 = 3.2;
const TRADE_MARKER_EDGE_PADDING: f32 = 1.0;
const TRADE_MARKER_MIN_PRICE_HEIGHT: f32 = (TRADE_MARKER_RADIUS + TRADE_MARKER_EDGE_PADDING) * 2.0;
const TRADE_MARKER_CANDLE_GAP: f32 = 8.0;
const TRADE_MARKER_STACK_GAP: f32 = 7.0;
const TRADE_MARKER_MAX_STACK: usize = 4;
const TRADE_MARKER_MAX_GROUPS: usize = 240;
const TRADE_MARKER_HOVER_DISTANCE: f32 = 8.0;

#[derive(Debug, Clone, PartialEq)]
struct TradeMarkerGroup {
    candle_idx: usize,
    is_buy: bool,
    count: usize,
    price: f64,
}

#[derive(Debug, Clone)]
struct TradeMarkerAccumulator {
    candle_idx: usize,
    is_buy: bool,
    count: usize,
    price_sum: f64,
    weighted_price_sum: f64,
    weight_sum: f64,
}

impl TradeMarkerAccumulator {
    fn new(candle_idx: usize, is_buy: bool) -> Self {
        Self {
            candle_idx,
            is_buy,
            count: 0,
            price_sum: 0.0,
            weighted_price_sum: 0.0,
            weight_sum: 0.0,
        }
    }

    fn add(&mut self, marker: &TradeMarker) {
        self.count += 1;
        self.price_sum += marker.price;
        if marker.size.is_finite() && marker.size > 0.0 {
            self.weighted_price_sum += marker.price * marker.size;
            self.weight_sum += marker.size;
        }
    }

    fn into_group(self) -> TradeMarkerGroup {
        let price = if self.weight_sum > 0.0 {
            self.weighted_price_sum / self.weight_sum
        } else if self.count > 0 {
            self.price_sum / self.count as f64
        } else {
            self.price_sum
        };

        TradeMarkerGroup {
            candle_idx: self.candle_idx,
            is_buy: self.is_buy,
            count: self.count,
            price,
        }
    }
}

impl CandlestickChart {
    pub(super) fn draw_trade_markers<PriceToY, IdxToCx>(
        &self,
        ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    ) where
        PriceToY: Fn(f64) -> f32,
        IdxToCx: Fn(usize) -> f32,
    {
        if !self.show_trade_markers || self.trade_markers.is_empty() || ctx.price_range <= 0.0 {
            return;
        }

        let groups = visible_trade_marker_groups(
            ctx.candles,
            &self.trade_markers,
            ctx.first_vis,
            ctx.last_vis,
        );
        if groups.is_empty() || ctx.price_h < TRADE_MARKER_MIN_PRICE_HEIGHT {
            return;
        }

        for group in &groups {
            self.draw_trade_marker_group(ctx, group);
        }

        self.draw_trade_marker_hover(ctx, &groups);
    }

    fn draw_trade_marker_group<PriceToY, IdxToCx>(
        &self,
        ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
        group: &TradeMarkerGroup,
    ) where
        PriceToY: Fn(f64) -> f32,
        IdxToCx: Fn(usize) -> f32,
    {
        let Some(layout) = trade_marker_layout(ctx, group) else {
            return;
        };

        let fill_color = Color {
            a: 0.92,
            ..if group.is_buy {
                ctx.theme.palette().success
            } else {
                ctx.theme.palette().danger
            }
        };
        let outline = Color {
            a: 0.86,
            ..ctx.theme.extended_palette().background.strong.color
        };
        for dot in layout.visible_dots() {
            let circle = canvas::Path::circle(dot.center, dot.radius);
            ctx.frame.fill(&circle, fill_color);
            ctx.frame.stroke(
                &circle,
                canvas::Stroke::default()
                    .with_color(outline)
                    .with_width(0.9),
            );
        }
    }

    fn draw_trade_marker_hover<PriceToY, IdxToCx>(
        &self,
        ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
        groups: &[TradeMarkerGroup],
    ) where
        PriceToY: Fn(f64) -> f32,
        IdxToCx: Fn(usize) -> f32,
    {
        let Some(pos) = ctx.state.cursor_position else {
            return;
        };
        if pos.x > ctx.chart_w || pos.y > ctx.price_h {
            return;
        }

        let Some((group, _dist)) = nearest_trade_marker_group(ctx, groups, pos) else {
            return;
        };

        let side = if group.is_buy {
            "Buy fills"
        } else {
            "Sell fills"
        };
        let side_color = if group.is_buy {
            ctx.theme.palette().success
        } else {
            ctx.theme.palette().danger
        };
        let candle_time = ctx
            .candles
            .get(group.candle_idx)
            .map(|candle| candle.open_time)
            .unwrap_or_default();
        let lines = vec![
            TooltipLine {
                content: side.to_string(),
                color: side_color,
            },
            TooltipLine {
                content: format!("Count: {}", group.count),
                color: ctx.theme.palette().text,
            },
            TooltipLine {
                content: format!("VWAP: {}", format_price(group.price)),
                color: Color {
                    a: 0.76,
                    ..ctx.theme.palette().text
                },
            },
            TooltipLine {
                content: format_timestamp_exact(candle_time),
                color: Color {
                    a: 0.62,
                    ..ctx.theme.palette().text
                },
            },
        ];

        let card_w: f32 = 142.0;
        let line_h: f32 = 14.0;
        let pad: f32 = 6.0;
        let card_h = lines.len() as f32 * line_h + pad * 2.0;
        let max_card_x = (ctx.chart_w - card_w - 4.0).max(4.0);
        let max_card_y = (ctx.price_h - card_h).max(0.0);
        let card_x = (pos.x + 14.0).min(max_card_x).max(4.0);
        let card_y = (pos.y - card_h - 8.0).clamp(0.0, max_card_y);
        let mut tooltip_surface =
            TooltipSurface::new(ctx.frame, ctx.theme, pos, ctx.chart_w, ctx.price_h);
        tooltip_surface.draw_card(
            Point::new(card_x, card_y),
            Size::new(card_w, card_h),
            &lines,
        );
    }
}

#[derive(Debug, Clone, Copy)]
struct TradeMarkerGroupLayout {
    dots: [TradeMarkerDot; TRADE_MARKER_MAX_STACK],
    dot_count: usize,
}

impl TradeMarkerGroupLayout {
    fn visible_dots(&self) -> &[TradeMarkerDot] {
        &self.dots[..self.dot_count]
    }
}

#[derive(Debug, Clone, Copy)]
struct TradeMarkerDot {
    center: Point,
    radius: f32,
}

fn trade_marker_layout<PriceToY, IdxToCx>(
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

fn trade_marker_clamp_bounds(price_h: f32) -> Option<(f32, f32)> {
    (price_h >= TRADE_MARKER_MIN_PRICE_HEIGHT).then_some((
        TRADE_MARKER_RADIUS + TRADE_MARKER_EDGE_PADDING,
        price_h - TRADE_MARKER_RADIUS - TRADE_MARKER_EDGE_PADDING,
    ))
}

fn trade_marker_dot_radius(_group_count: usize, _dot_count: usize, _stack_idx: usize) -> f32 {
    TRADE_MARKER_RADIUS
}

fn trade_marker_anchor_y<PriceToY>(
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

fn nearest_trade_marker_group<'a, PriceToY, IdxToCx>(
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

fn visible_trade_marker_groups(
    candles: &[Candle],
    markers: &[TradeMarker],
    first_vis: usize,
    last_vis: usize,
) -> Vec<TradeMarkerGroup> {
    if candles.is_empty() || markers.is_empty() || first_vis > last_vis || last_vis >= candles.len()
    {
        return Vec::new();
    }

    let visible_count = last_vis - first_vis + 1;
    let per_side_limit = (TRADE_MARKER_MAX_GROUPS / 2).max(1);
    let stride = visible_count.div_ceil(per_side_limit).max(1);
    let mut grouped: BTreeMap<(usize, bool), TradeMarkerAccumulator> = BTreeMap::new();

    let visible_start = candles[first_vis].open_time;
    let visible_end = candles[last_vis].close_time;
    let first_marker = markers.partition_point(|marker| marker.time_ms < visible_start);

    for marker in &markers[first_marker..] {
        if marker.time_ms > visible_end {
            break;
        }
        if !marker.price.is_finite() || marker.price <= 0.0 {
            continue;
        }

        let Some(candle_idx) = candle_index_for_time(candles, marker.time_ms) else {
            continue;
        };
        if candle_idx < first_vis || candle_idx > last_vis {
            continue;
        }

        let bucket_idx = bucket_candle_index(candle_idx, first_vis, last_vis, stride);
        grouped
            .entry((bucket_idx, marker.is_buy))
            .or_insert_with(|| TradeMarkerAccumulator::new(bucket_idx, marker.is_buy))
            .add(marker);
    }

    grouped
        .into_values()
        .filter(|group| group.count > 0)
        .map(TradeMarkerAccumulator::into_group)
        .collect()
}

fn candle_index_for_time(candles: &[Candle], time_ms: u64) -> Option<usize> {
    let idx = match candles.binary_search_by_key(&time_ms, |candle| candle.open_time) {
        Ok(idx) => idx,
        Err(0) => return None,
        Err(idx) => idx.saturating_sub(1),
    };

    candles.get(idx).and_then(|candle| {
        (time_ms >= candle.open_time && time_ms <= candle.close_time).then_some(idx)
    })
}

fn bucket_candle_index(
    candle_idx: usize,
    first_vis: usize,
    last_vis: usize,
    stride: usize,
) -> usize {
    if stride <= 1 {
        return candle_idx;
    }

    let bucket_start = first_vis + ((candle_idx - first_vis) / stride) * stride;
    bucket_start
        .saturating_add(stride / 2)
        .min(last_vis)
        .max(first_vis)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(open_time: u64) -> Candle {
        Candle {
            open_time,
            close_time: open_time + 59_999,
            open: 100.0,
            high: 110.0,
            low: 90.0,
            close: 105.0,
            volume: 1.0,
        }
    }

    fn marker(time_ms: u64, price: f64, size: f64, is_buy: bool) -> TradeMarker {
        TradeMarker {
            time_ms,
            price,
            size,
            is_buy,
        }
    }

    #[test]
    fn groups_multiple_fills_on_the_same_candle_and_side() {
        let candles = vec![candle(0), candle(60_000), candle(120_000)];
        let markers = vec![
            marker(5_000, 100.0, 1.0, true),
            marker(10_000, 110.0, 3.0, true),
            marker(70_000, 120.0, 1.0, false),
        ];

        let groups = visible_trade_marker_groups(&candles, &markers, 0, 2);

        assert_eq!(groups.len(), 2);
        let buy = groups.iter().find(|group| group.is_buy).expect("buy group");
        assert_eq!(buy.candle_idx, 0);
        assert_eq!(buy.count, 2);
        assert!((buy.price - 107.5).abs() < f64::EPSILON);

        let sell = groups
            .iter()
            .find(|group| !group.is_buy)
            .expect("sell group");
        assert_eq!(sell.candle_idx, 1);
        assert_eq!(sell.count, 1);
    }

    #[test]
    fn coarsens_dense_visible_history_into_limited_groups() {
        let candles: Vec<_> = (0..400).map(|idx| candle(idx * 60_000)).collect();
        let markers: Vec<_> = candles
            .iter()
            .map(|candle| marker(candle.open_time + 1_000, 100.0, 1.0, true))
            .collect();

        let groups = visible_trade_marker_groups(&candles, &markers, 0, candles.len() - 1);

        assert!(groups.len() < markers.len());
        assert!(groups.len() <= TRADE_MARKER_MAX_GROUPS);
        assert_eq!(groups.iter().map(|group| group.count).sum::<usize>(), 400);
    }

    #[test]
    fn skips_markers_outside_visible_candles_or_invalid_prices() {
        let candles = vec![candle(60_000), candle(120_000)];
        let markers = vec![
            marker(10_000, 100.0, 1.0, true),
            marker(65_000, -1.0, 1.0, true),
            marker(125_000, 101.0, 1.0, true),
            marker(240_000, 100.0, 1.0, false),
        ];

        let groups = visible_trade_marker_groups(&candles, &markers, 0, 1);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].candle_idx, 1);
        assert_eq!(groups[0].count, 1);
    }

    #[test]
    fn marker_clamp_bounds_reject_tiny_price_areas() {
        assert!(trade_marker_clamp_bounds(TRADE_MARKER_MIN_PRICE_HEIGHT - 0.1).is_none());

        let (min_y, max_y) =
            trade_marker_clamp_bounds(TRADE_MARKER_MIN_PRICE_HEIGHT).expect("valid bounds");
        assert!(min_y <= max_y);
    }

    #[test]
    fn grouped_fill_dots_use_uniform_radius() {
        assert_eq!(trade_marker_dot_radius(4, 4, 3), TRADE_MARKER_RADIUS);
        assert_eq!(trade_marker_dot_radius(5, 4, 0), TRADE_MARKER_RADIUS);
        assert_eq!(trade_marker_dot_radius(5, 4, 3), TRADE_MARKER_RADIUS);
    }

    #[test]
    fn marker_anchor_keeps_dots_away_from_candle_edges() {
        let candle = candle(0);
        let price_to_y = |price: f64| (120.0 - price) as f32;

        let buy_y = trade_marker_anchor_y(&candle, true, &price_to_y).expect("buy anchor");
        let sell_y = trade_marker_anchor_y(&candle, false, &price_to_y).expect("sell anchor");

        assert!(buy_y > price_to_y(candle.low) + TRADE_MARKER_CANDLE_GAP);
        assert!(sell_y < price_to_y(candle.high) - TRADE_MARKER_CANDLE_GAP);
    }

    #[test]
    fn marker_anchor_uses_visual_edges_when_axis_is_inverted() {
        let candle = candle(0);
        let inverted_price_to_y = |price: f64| price as f32;

        let buy_y = trade_marker_anchor_y(&candle, true, &inverted_price_to_y).expect("buy anchor");
        let sell_y =
            trade_marker_anchor_y(&candle, false, &inverted_price_to_y).expect("sell anchor");

        assert!(buy_y > inverted_price_to_y(candle.high) + TRADE_MARKER_CANDLE_GAP);
        assert!(sell_y < inverted_price_to_y(candle.low) - TRADE_MARKER_CANDLE_GAP);
    }
}

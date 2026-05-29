use super::TradingOverlayContext;
use crate::chart::model::CandlestickChart;
use crate::chart::tooltips::{TooltipLine, TooltipSurface};
use crate::helpers::{format_price, format_timestamp_exact};
use iced::widget::canvas;
use iced::{Color, Point};

mod grouping;
mod layout;

#[cfg(test)]
use grouping::TRADE_MARKER_MAX_GROUPS;
use grouping::{TradeMarkerGroup, visible_trade_marker_groups};
#[cfg(test)]
use layout::{
    TRADE_MARKER_CANDLE_GAP, TRADE_MARKER_RADIUS, trade_marker_anchor_y, trade_marker_clamp_bounds,
    trade_marker_dot_radius,
};
use layout::{TRADE_MARKER_MIN_PRICE_HEIGHT, nearest_trade_marker_group, trade_marker_layout};

// ---------------------------------------------------------------------------
// Trade Marker Overlays
// ---------------------------------------------------------------------------

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
            ctx.fisheye
                .fill_projected_circle(ctx.frame, dot.center, dot.radius, fill_color);
            ctx.fisheye.stroke_projected_circle(
                ctx.frame,
                dot.center,
                dot.radius,
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
        let visual_pos = ctx.fisheye.project(pos);
        if visual_pos.x > ctx.chart_w || visual_pos.y > ctx.price_h {
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

        let card_size = TooltipSurface::card_size_for_lines(&lines, 142.0);
        let max_card_x = (ctx.chart_w - card_size.width - 4.0).max(4.0);
        let max_card_y = (ctx.price_h - card_size.height).max(0.0);
        let card_x = (visual_pos.x + 14.0).min(max_card_x).max(4.0);
        let card_y = (visual_pos.y - card_size.height - 8.0).clamp(0.0, max_card_y);
        let mut tooltip_surface =
            TooltipSurface::new(ctx.frame, ctx.theme, visual_pos, ctx.chart_w, ctx.price_h);
        tooltip_surface.draw_card(Point::new(card_x, card_y), card_size, &lines);
    }
}

#[cfg(test)]
mod tests;

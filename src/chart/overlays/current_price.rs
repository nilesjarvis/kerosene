use super::TradingOverlayContext;
use crate::chart::candle_layer::line_series_stroke_color;
use crate::chart::drawing::{
    AxisBadgeStyle, SegmentedHLineStyle, stroke_projected_segmented_hline_with_offset,
};
use crate::chart::model::CandlestickChart;
use crate::chart::price_badges::{
    RIGHT_AXIS_PRIMARY_BADGE_HEIGHT, RightAxisBadgeConnectorStyle, RightAxisBadgeKind,
    draw_stacked_right_axis_badge, right_axis_line_end_x,
};
use crate::helpers::{format_price, text_color_for_bg};
use iced::{Color, Theme};

// ---------------------------------------------------------------------------
// Current Price Overlay
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn draw_current_price_line<PriceToY, IdxToCx>(
        &self,
        ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    ) where
        PriceToY: Fn(f64) -> f32,
        IdxToCx: Fn(usize) -> f32,
    {
        if let Some(last_candle) = self.candles.last()
            && ctx.price_range > 0.0
        {
            let last_px = last_candle.close;
            let last_y = (ctx.price_to_y)(last_px);

            if last_y >= -10.0 && last_y <= ctx.price_h + 10.0 {
                let is_bullish = last_candle.close >= last_candle.open;
                let (price_color, text_color) = current_price_badge_colors(
                    self,
                    is_bullish,
                    ctx.theme,
                    ctx.candle_bull_color,
                    ctx.candle_bear_color,
                );
                let price_color_dim = Color {
                    a: 0.35,
                    ..price_color
                };

                let badge_kind = RightAxisBadgeKind::CurrentPrice;
                let line_end_x =
                    right_axis_line_end_x(ctx.right_axis_badges, badge_kind, ctx.chart_w);
                stroke_projected_segmented_hline_with_offset(
                    ctx.frame,
                    ctx.fisheye,
                    line_end_x,
                    last_y,
                    SegmentedHLineStyle {
                        segment_len: 2.0,
                        gap_len: 3.0,
                        offset: 0.0,
                        color: price_color_dim,
                        width: 1.0,
                    },
                );
                draw_stacked_right_axis_badge(
                    ctx.frame,
                    ctx.right_axis_badges,
                    badge_kind,
                    ctx.chart_w,
                    last_y,
                    format_price(last_px),
                    price_color,
                    AxisBadgeStyle {
                        char_width: 6.5,
                        padding_width: 8.0,
                        height: RIGHT_AXIS_PRIMARY_BADGE_HEIGHT,
                        text_size: 10.0,
                        text_color,
                    },
                    RightAxisBadgeConnectorStyle::Segmented {
                        style: SegmentedHLineStyle {
                            segment_len: 2.0,
                            gap_len: 3.0,
                            offset: 0.0,
                            color: price_color_dim,
                            width: 1.0,
                        },
                    },
                    ctx.fisheye,
                );
            }
        }
    }
}

pub(super) fn current_price_badge_colors(
    chart: &CandlestickChart,
    is_bullish: bool,
    theme: &Theme,
    candle_bull_color: Color,
    candle_bear_color: Color,
) -> (Color, Color) {
    if chart.series_style.is_line() {
        let color = line_series_stroke_color(chart, theme);
        (color, text_color_for_bg(color))
    } else if is_bullish {
        (candle_bull_color, Color::BLACK)
    } else {
        (candle_bear_color, Color::BLACK)
    }
}

use super::CandleLayerContext;
use crate::chart::model::CandlestickChart;
use iced::widget::canvas;
use iced::{Color, Point, Size};

// ---------------------------------------------------------------------------
// Candle and Volume Rendering
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn draw_candles<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        for i in ctx.first_vis..=ctx.last_vis {
            let candle = &self.candles[i];
            let cx = (ctx.idx_to_cx)(i);

            if cx + ctx.candle_w * 0.5 < 0.0 || cx - ctx.candle_w * 0.5 > ctx.chart_w {
                continue;
            }

            let is_bullish = candle.close >= candle.open;
            let color = if is_bullish {
                ctx.candle_bull_color
            } else {
                ctx.candle_bear_color
            };

            let open_y = (ctx.price_to_y)(candle.open);
            let close_y = (ctx.price_to_y)(candle.close);
            let body_top = open_y.min(close_y);
            let body_h = (open_y - close_y).abs().max(1.0);
            let body_bottom = body_top + body_h;
            let high_y = (ctx.price_to_y)(candle.high);
            let low_y = (ctx.price_to_y)(candle.low);
            let wick_top = high_y.min(low_y);
            let wick_bottom = high_y.max(low_y);
            if wick_top < body_top {
                ctx.fisheye.stroke_projected_line_without_edge_blur(
                    frame,
                    Point::new(cx, wick_top),
                    Point::new(cx, body_top),
                    canvas::Stroke::default().with_color(color).with_width(1.0),
                );
            }
            if body_bottom < wick_bottom {
                ctx.fisheye.stroke_projected_line_without_edge_blur(
                    frame,
                    Point::new(cx, body_bottom),
                    Point::new(cx, wick_bottom),
                    canvas::Stroke::default().with_color(color).with_width(1.0),
                );
            }
            let body_origin = Point::new(cx - ctx.candle_w * 0.5, body_top);
            let body_size = Size::new(ctx.candle_w, body_h);
            if self.hollow_candle_mode.applies_to(is_bullish) {
                ctx.fisheye.stroke_projected_rect_without_edge_blur(
                    frame,
                    body_origin,
                    body_size,
                    canvas::Stroke::default().with_color(color).with_width(1.25),
                );
            } else {
                ctx.fisheye.fill_projected_rect_without_edge_blur(
                    frame,
                    body_origin,
                    body_size,
                    color,
                );
            }
        }
    }

    pub(super) fn draw_volume_bars<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        for i in ctx.first_vis..=ctx.last_vis {
            let candle = &self.candles[i];
            let cx = (ctx.idx_to_cx)(i);

            if cx + ctx.candle_w * 0.5 < 0.0 || cx - ctx.candle_w * 0.5 > ctx.chart_w {
                continue;
            }

            let is_bullish = candle.close >= candle.open;
            let vol_frac = if ctx.vol_max > 0.0 {
                (candle.volume / ctx.vol_max) as f32
            } else {
                0.0
            };
            let bar_h = vol_frac * (ctx.volume_h - 4.0);
            let vol_color = if is_bullish {
                Color {
                    a: 0.35,
                    ..ctx.candle_bull_color
                }
            } else {
                Color {
                    a: 0.35,
                    ..ctx.candle_bear_color
                }
            };
            ctx.fisheye.fill_projected_rect_without_edge_blur(
                frame,
                Point::new(cx - ctx.candle_w * 0.5, ctx.price_h + ctx.volume_h - bar_h),
                Size::new(ctx.candle_w, bar_h),
                vol_color,
            );
        }
    }
}

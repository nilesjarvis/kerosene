use super::CandleLayerContext;
use crate::chart::model::CandlestickChart;
use iced::widget::canvas;
use iced::{Color, Point, Size};

// ---------------------------------------------------------------------------
// Candle and Volume Rendering
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn draw_candles_and_volume<IdxToCx, PriceToY>(
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

            let wick_top = (ctx.price_to_y)(candle.high);
            let wick_bot = (ctx.price_to_y)(candle.low);
            ctx.fisheye.stroke_projected_line(
                frame,
                Point::new(cx, wick_top),
                Point::new(cx, wick_bot),
                canvas::Stroke::default().with_color(color).with_width(1.0),
            );

            let open_y = (ctx.price_to_y)(candle.open);
            let close_y = (ctx.price_to_y)(candle.close);
            let body_top = open_y.min(close_y);
            let body_h = (open_y - close_y).abs().max(1.0);
            ctx.fisheye.fill_projected_rect(
                frame,
                Point::new(cx - ctx.candle_w * 0.5, body_top),
                Size::new(ctx.candle_w, body_h),
                color,
            );

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
            ctx.fisheye.fill_projected_rect(
                frame,
                Point::new(cx - ctx.candle_w * 0.5, ctx.price_h + ctx.volume_h - bar_h),
                Size::new(ctx.candle_w, bar_h),
                vol_color,
            );
        }
    }
}

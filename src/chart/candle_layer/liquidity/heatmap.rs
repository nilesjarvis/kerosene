use super::super::CandleLayerContext;
use crate::chart::model::CandlestickChart;
use iced::widget::canvas;
use iced::{Color, Point, Size};

// ---------------------------------------------------------------------------
// Heatmap Rendering
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(in crate::chart::candle_layer) fn draw_historical_heatmap<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        if self.heatmap_rects.is_empty() || self.heatmap_max_usd <= 0.0 || ctx.price_range <= 0.0 {
            return;
        }

        for rect in self.heatmap_rects.iter().step_by(ctx.heatmap_stride) {
            let Some((x_left, x_right)) =
                self.heatmap_x_bounds(rect, ctx.state, ctx.chart_w, ctx.step)
            else {
                continue;
            };
            if x_right < 0.0 || x_left > ctx.chart_w {
                continue;
            }

            let x_left = x_left.max(0.0);
            let x_right = x_right.min(ctx.chart_w);
            let cell_w = (x_right - x_left).max(1.0);
            let y_top = (ctx.price_to_y)(rect.price_hi);
            let y_bot = (ctx.price_to_y)(rect.price_lo);
            if y_top > ctx.price_h || y_bot < 0.0 {
                continue;
            }
            let y_top = y_top.max(0.0);
            let y_bot = y_bot.min(ctx.price_h);
            let cell_h = (y_bot - y_top).max(1.0);

            let intensity = (rect.amount_usd.abs() / self.heatmap_max_usd) as f32;
            let alpha = intensity.sqrt().clamp(0.0, 1.0) * 0.55;
            if alpha < 0.005 {
                continue;
            }

            let color = if rect.amount_usd >= 0.0 {
                Color {
                    a: alpha,
                    ..ctx.theme.palette().success
                }
            } else {
                Color {
                    a: alpha,
                    ..ctx.theme.palette().danger
                }
            };

            frame.fill_rectangle(Point::new(x_left, y_top), Size::new(cell_w, cell_h), color);
        }
    }
}

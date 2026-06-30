use super::super::CandleLayerContext;
use crate::chart::model::CandlestickChart;
use crate::chart::model::heatmap_stride_for_visible_count;
use crate::hyperdash_api::HeatmapRect;
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

        let visible_count = self
            .heatmap_rects
            .iter()
            .filter(|rect| self.clipped_heatmap_rect_bounds(rect, ctx).is_some())
            .count();
        let stride = heatmap_stride_for_visible_count(visible_count, ctx.heatmap_rect_budget);

        let mut visible_index = 0;
        for rect in &self.heatmap_rects {
            let Some((x_left, x_right, y_top, y_bot)) = self.clipped_heatmap_rect_bounds(rect, ctx)
            else {
                continue;
            };
            let should_draw = visible_index % stride == 0;
            visible_index += 1;
            if !should_draw {
                continue;
            }

            let cell_w = (x_right - x_left).max(1.0);
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

            ctx.fisheye.fill_projected_rect_flat(
                frame,
                Point::new(x_left, y_top),
                Size::new(cell_w, cell_h),
                color,
            );
        }
    }

    fn clipped_heatmap_rect_bounds<IdxToCx, PriceToY>(
        &self,
        rect: &HeatmapRect,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
    ) -> Option<(f32, f32, f32, f32)>
    where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let (x_left, x_right) = self.heatmap_x_bounds(rect, ctx.state, ctx.chart_w, ctx.step)?;
        if x_right < 0.0 || x_left > ctx.chart_w {
            return None;
        }

        let y_top = (ctx.price_to_y)(rect.price_hi);
        let y_bot = (ctx.price_to_y)(rect.price_lo);
        if y_top > ctx.price_h || y_bot < 0.0 {
            return None;
        }

        Some((
            x_left.max(0.0),
            x_right.min(ctx.chart_w),
            y_top.max(0.0),
            y_bot.min(ctx.price_h),
        ))
    }
}

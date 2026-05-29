use super::super::CandleLayerContext;
use crate::chart::model::CandlestickChart;
use iced::widget::canvas;
use iced::{Color, Point, Size};

// ---------------------------------------------------------------------------
// Liquidation Bucket Rendering
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(in crate::chart::candle_layer) fn draw_liquidation_bucket_bars<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        if self.liquidation_buckets.is_empty() || ctx.price_range <= 0.0 {
            return;
        }

        let max_usd = self
            .liquidation_buckets
            .iter()
            .map(|b| b.long_usd.max(b.short_usd))
            .fold(0.0_f64, f64::max);
        if max_usd <= 0.0 {
            return;
        }

        let max_bar_w = ctx.chart_w * 0.25;
        let bucket_count = self.liquidation_buckets.len();
        let bucket_h = ctx.price_h / bucket_count as f32;

        for bucket in &self.liquidation_buckets {
            let y = (ctx.price_to_y)(bucket.price_center);
            if y < -bucket_h || y > ctx.price_h + bucket_h {
                continue;
            }

            if bucket.long_usd > 0.0 {
                let frac = (bucket.long_usd / max_usd) as f32;
                let bar_w = frac * max_bar_w;
                ctx.fisheye.fill_projected_rect(
                    frame,
                    Point::new(ctx.chart_w - bar_w, y - bucket_h * 0.5),
                    Size::new(bar_w, bucket_h.max(1.5)),
                    Color {
                        a: 0.15,
                        ..ctx.theme.palette().success
                    },
                );
            }

            if bucket.short_usd > 0.0 {
                let frac = (bucket.short_usd / max_usd) as f32;
                let bar_w = frac * max_bar_w;
                ctx.fisheye.fill_projected_rect(
                    frame,
                    Point::new(ctx.chart_w - bar_w, y - bucket_h * 0.5),
                    Size::new(bar_w, bucket_h.max(1.5)),
                    Color {
                        a: 0.15,
                        ..ctx.theme.palette().danger
                    },
                );
            }
        }
    }
}

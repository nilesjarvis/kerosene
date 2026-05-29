use super::super::CandleLayerContext;
use crate::chart::model::CandlestickChart;
use crate::chart::volume_profile::{calculate_volume_profile, volume_profile_bucket_count};
use iced::widget::canvas;
use iced::{Color, Point, Size};

// ---------------------------------------------------------------------------
// Volume Profile Rendering
// ---------------------------------------------------------------------------

const VOLUME_PROFILE_MAX_WIDTH_RATIO: f32 = 0.24;
const VOLUME_PROFILE_MAX_WIDTH: f32 = 180.0;

impl CandlestickChart {
    pub(in crate::chart::candle_layer) fn draw_volume_profile<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        if !self.macro_indicators.show_volume_profile || ctx.price_range <= 0.0 {
            return;
        }

        let bucket_count = volume_profile_bucket_count(ctx.price_h);
        let Some(profile) = calculate_volume_profile(
            &self.candles[ctx.first_vis..=ctx.last_vis],
            ctx.price_lo,
            ctx.price_hi,
            bucket_count,
        ) else {
            return;
        };

        let max_bar_w = if ctx.chart_w.is_nan() {
            0.0
        } else {
            (ctx.chart_w * VOLUME_PROFILE_MAX_WIDTH_RATIO).clamp(0.0, VOLUME_PROFILE_MAX_WIDTH)
        };
        if max_bar_w <= 0.0 {
            return;
        }

        for bucket in profile.buckets {
            if bucket.volume <= 0.0 {
                continue;
            }

            let y_a = (ctx.price_to_y)(bucket.price_hi);
            let y_b = (ctx.price_to_y)(bucket.price_lo);
            let y_top = y_a.min(y_b).max(0.0);
            let y_bottom = y_a.max(y_b).min(ctx.price_h);
            if y_bottom <= 0.0 || y_top >= ctx.price_h || y_bottom <= y_top {
                continue;
            }

            let frac = (bucket.volume / profile.max_volume) as f32;
            let bar_w = (frac.sqrt().clamp(0.0, 1.0)) * max_bar_w;
            if bar_w <= 0.0 {
                continue;
            }

            let alpha = if (bucket.volume - profile.max_volume).abs() <= f64::EPSILON {
                0.24
            } else {
                0.15
            };
            ctx.fisheye.fill_projected_rect(
                frame,
                Point::new(ctx.chart_w - bar_w, y_top),
                Size::new(bar_w, (y_bottom - y_top).max(1.0)),
                Color {
                    a: alpha,
                    ..ctx.theme.palette().primary
                },
            );
        }
    }
}

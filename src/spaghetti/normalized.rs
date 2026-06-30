use super::helpers::find_candle_at;
use super::{PRICE_PADDING_PCT, Series, SpaghettiCanvas, SpaghettiChartState};
use crate::chart_background::{draw_dotted_background, draw_gradient_background};
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Theme};

mod axes;
mod crosshair;
mod series;

// ---------------------------------------------------------------------------
// Normalized Percentage Rendering
// ---------------------------------------------------------------------------

pub(super) struct NormalizedRenderContext<'a> {
    pub(super) state: &'a SpaghettiChartState,
    pub(super) renderer: &'a Renderer,
    pub(super) theme: &'a Theme,
    pub(super) bounds: Rectangle,
    pub(super) chart_w: f32,
    pub(super) chart_h: f32,
    pub(super) left_ts: f64,
    pub(super) right_ts: f64,
    pub(super) visible_ms: f64,
    pub(super) time_px_per_ms: f64,
    pub(super) effective_max: u64,
    pub(super) base_ts: u64,
    pub(super) crosshair_style: crate::config::ChartCrosshairStyle,
    pub(super) crosshair_guides_enabled: bool,
    pub(super) crosshair_scale: f32,
}

impl SpaghettiCanvas {
    pub(super) fn draw_normalized(
        &self,
        ctx: NormalizedRenderContext<'_>,
        loaded_series: &[&Series],
    ) -> Vec<canvas::Geometry> {
        let ts_to_x = |ts: u64| -> f32 { ((ts as f64 - ctx.left_ts) * ctx.time_px_per_ms) as f32 };

        let series_data: Vec<(&Series, Vec<(f32, f64)>)> = loaded_series
            .iter()
            .filter_map(|s| {
                let base_idx = find_candle_at(&s.candles, ctx.base_ts)?;
                let base_price = s.candles[base_idx].close;
                if base_price <= 0.0 {
                    return None;
                }
                let points: Vec<(f32, f64)> = s
                    .candles
                    .iter()
                    .filter(|c| {
                        (c.open_time as f64) >= ctx.left_ts && (c.open_time as f64) <= ctx.right_ts
                    })
                    .map(|c| {
                        let x = ts_to_x(c.open_time);
                        let pct = (c.close / base_price - 1.0) * 100.0;
                        (x, pct)
                    })
                    .collect();
                if points.is_empty() {
                    return None;
                }
                Some((*s, points))
            })
            .collect();

        if series_data.is_empty() {
            return vec![];
        }

        let (auto_lo, auto_hi) = series_data
            .iter()
            .flat_map(|(_, pts)| pts.iter().map(|(_, p)| *p))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(p), hi.max(p))
            });
        let pad = (auto_hi - auto_lo).max(1.0) * PRICE_PADDING_PCT;
        let auto_lo = auto_lo - pad;
        let auto_hi = auto_hi + pad;

        let (pct_lo, pct_hi) = if ctx.state.y_auto {
            (auto_lo, auto_hi)
        } else {
            let range = (auto_hi - auto_lo) * ctx.state.y_scale;
            let mid = (auto_hi + auto_lo) * 0.5 + ctx.state.y_offset;
            (mid - range * 0.5, mid + range * 0.5)
        };
        let pct_range = (pct_hi - pct_lo).max(0.01);
        let pct_to_y =
            |pct: f64| -> f32 { ((pct_hi - pct) / pct_range * ctx.chart_h as f64) as f32 };

        let mut frame = canvas::Frame::new(ctx.renderer, ctx.bounds.size());
        frame.fill_rectangle(Point::ORIGIN, ctx.bounds.size(), Color::TRANSPARENT);

        if self.gradient_background {
            draw_gradient_background(&mut frame, ctx.theme, ctx.chart_w, ctx.chart_h);
        }
        if self.dotted_background {
            draw_dotted_background(
                &mut frame,
                ctx.theme,
                ctx.chart_w,
                ctx.chart_h,
                self.dotted_background_opacity,
                crate::chart::fisheye::ChartFisheye::disabled(),
            );
        }
        axes::draw_grid_and_axes(
            &mut frame,
            &ctx,
            pct_hi,
            pct_range,
            &pct_to_y,
            !self.dotted_background,
        );
        series::draw_session_start_line(&mut frame, &ctx, &ts_to_x, self.base_timestamp);
        series::draw_series_lines(&mut frame, &ctx, &series_data, &pct_to_y, self.color_mode);
        if self.effective_show_labels() {
            series::draw_series_labels(&mut frame, &ctx, &series_data, &pct_to_y, self.color_mode);
        }
        series::draw_legend(&mut frame, ctx.theme, self.color_mode, &series_data);

        let base_geo = frame.into_geometry();
        let overlay_geo = crosshair::draw_crosshair_overlay(&ctx, pct_hi, pct_range);

        vec![base_geo, overlay_geo]
    }
}

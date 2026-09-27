use super::helpers::find_candle_at;
use super::{PRICE_PADDING_PCT, RenderContext, Series, SpaghettiCanvas};
use super::{axes, crosshair};
use iced::widget::canvas;
use iced::{Color, Point};

mod series;

// ---------------------------------------------------------------------------
// Normalized Percentage Rendering
// ---------------------------------------------------------------------------

impl SpaghettiCanvas {
    pub(super) fn draw_normalized(
        &self,
        ctx: RenderContext<'_>,
        loaded_series: &[&Series],
        base_ts: u64,
    ) -> Vec<canvas::Geometry> {
        let ts_to_x = |ts: u64| -> f32 { ((ts as f64 - ctx.left_ts) * ctx.time_px_per_ms) as f32 };

        let series_data: Vec<(&Series, Vec<(f32, f64)>)> = loaded_series
            .iter()
            .filter_map(|s| {
                let base_idx = find_candle_at(&s.candles, base_ts)?;
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

        let mut frame = self.background_frame(&ctx);
        axes::draw_value_grid(
            &mut frame,
            &ctx,
            pct_hi,
            pct_range,
            !self.dotted_background,
            |value| format!("{value:+.1}%"),
        );
        let zero_y = pct_to_y(0.0);
        if zero_y >= 0.0 && zero_y <= ctx.chart_h {
            let baseline =
                canvas::Path::line(Point::new(0.0, zero_y), Point::new(ctx.chart_w, zero_y));
            frame.stroke(
                &baseline,
                canvas::Stroke::default()
                    .with_color(Color {
                        a: 0.15,
                        ..ctx.theme.palette().text
                    })
                    .with_width(1.0),
            );
        }
        axes::draw_value_axis_border(&mut frame, &ctx);
        axes::draw_time_axis(&mut frame, &ctx);
        axes::draw_session_start_line(&mut frame, &ctx, &ts_to_x, self.base_timestamp);
        series::draw_series_lines(&mut frame, &ctx, &series_data, &pct_to_y, self.color_mode);
        if self.effective_show_labels() {
            series::draw_series_labels(&mut frame, &ctx, &series_data, &pct_to_y, self.color_mode);
        }
        series::draw_legend(&mut frame, ctx.theme, self.color_mode, &series_data);

        let base_geo = frame.into_geometry();
        let overlay_geo = crosshair::draw_crosshair_overlay(&ctx, |y| {
            (pct_range > 0.0).then(|| {
                let hover_pct = pct_hi - (y as f64 / ctx.chart_h as f64) * pct_range;
                format!("{hover_pct:+.2}%")
            })
        });

        vec![base_geo, overlay_geo]
    }
}

use super::NormalizedRenderContext;
use crate::spaghetti::{ComparisonColorMode, Series};

use iced::widget::canvas;
use iced::{Color, Point, Theme};

mod labels;
mod legend;

pub(super) use labels::draw_series_labels;
pub(super) use legend::draw_legend;

#[cfg(test)]
pub(super) use labels::{SeriesLabelAnchor, performance_label, stack_series_label_positions};
#[cfg(test)]
pub(super) use legend::legend_label;

// ---------------------------------------------------------------------------
// Normalized Series
// ---------------------------------------------------------------------------

pub(super) fn draw_session_start_line(
    frame: &mut canvas::Frame,
    ctx: &NormalizedRenderContext<'_>,
    ts_to_x: &impl Fn(u64) -> f32,
    base_timestamp: Option<u64>,
) {
    if let Some(base_ts) = base_timestamp {
        let base_x = ts_to_x(base_ts);
        if base_x >= 0.0 && base_x <= ctx.chart_w {
            let dash_len: f32 = 4.0;
            let gap_len: f32 = 3.0;
            let mut y = 0.0_f32;
            while y < ctx.chart_h {
                let end = (y + dash_len).min(ctx.chart_h);
                let seg = canvas::Path::line(Point::new(base_x, y), Point::new(base_x, end));
                frame.stroke(
                    &seg,
                    canvas::Stroke::default()
                        .with_color(Color {
                            a: 0.2,
                            ..ctx.theme.palette().text
                        })
                        .with_width(1.0),
                );
                y += dash_len + gap_len;
            }
        }
    }
}

pub(super) fn draw_series_lines(
    frame: &mut canvas::Frame,
    ctx: &NormalizedRenderContext<'_>,
    series_data: &[(&Series, Vec<(f32, f64)>)],
    pct_to_y: &impl Fn(f64) -> f32,
    color_mode: ComparisonColorMode,
) {
    for (series, points) in series_data {
        if points.len() < 2 {
            continue;
        }
        let mut path = canvas::path::Builder::new();
        for (idx, (x, pct)) in points.iter().enumerate() {
            let px = x.clamp(-10.0, ctx.chart_w + 10.0);
            let py = pct_to_y(*pct).clamp(-50.0, ctx.chart_h + 50.0);
            if idx == 0 {
                path.move_to(Point::new(px, py));
            } else {
                path.line_to(Point::new(px, py));
            }
        }
        frame.stroke(
            &path.build(),
            canvas::Stroke::default()
                .with_color(series_render_color(ctx.theme, color_mode, series))
                .with_width(1.5),
        );
    }
}

fn series_render_color(theme: &Theme, color_mode: ComparisonColorMode, series: &Series) -> Color {
    match color_mode {
        ComparisonColorMode::Multi => series.color,
        ComparisonColorMode::Single => crate::spaghetti::SpaghettiCanvas::single_color(theme),
    }
}

#[cfg(test)]
mod tests;

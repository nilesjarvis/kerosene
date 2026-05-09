use super::NormalizedRenderContext;
use crate::spaghetti::Series;
use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point, Size};

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
                .with_color(series.color)
                .with_width(1.5),
        );
    }
}

pub(super) fn draw_legend(frame: &mut canvas::Frame, series_data: &[(&Series, Vec<(f32, f64)>)]) {
    let mut legend_y = 8.0_f32;
    for (series, points) in series_data {
        frame.fill_rectangle(Point::new(8.0, legend_y), Size::new(8.0, 8.0), series.color);
        frame.fill_text(canvas::Text {
            content: legend_label(&series.display, points),
            position: Point::new(20.0, legend_y + 4.0),
            color: series.color,
            size: iced::Pixels(10.0),
            align_x: alignment::Horizontal::Left.into(),
            align_y: alignment::Vertical::Center,
            font: iced::Font::MONOSPACE,
            ..canvas::Text::default()
        });
        legend_y += 14.0;
    }
}

fn legend_label(display: &str, points: &[(f32, f64)]) -> String {
    points
        .last()
        .map(|(_, pct)| format!("{display} {pct:+.2}%"))
        .unwrap_or_else(|| format!("{display} --"))
}

#[cfg(test)]
mod tests {
    use super::legend_label;

    #[test]
    fn legend_label_marks_empty_series_unavailable() {
        assert_eq!(legend_label("BTC", &[]), "BTC --");
    }

    #[test]
    fn legend_label_formats_latest_percent() {
        assert_eq!(legend_label("BTC", &[(1.0, -2.5)]), "BTC -2.50%");
    }
}

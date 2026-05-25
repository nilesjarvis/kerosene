use super::series_render_color;
use crate::spaghetti::{ComparisonColorMode, Series};

use iced::alignment;
use iced::widget::canvas;
use iced::{Point, Size, Theme};

// ---------------------------------------------------------------------------
// Series Legend
// ---------------------------------------------------------------------------

pub(in crate::spaghetti::normalized) fn draw_legend(
    frame: &mut canvas::Frame,
    theme: &Theme,
    color_mode: ComparisonColorMode,
    series_data: &[(&Series, Vec<(f32, f64)>)],
) {
    let mut legend_y = 8.0_f32;
    for (series, points) in series_data {
        let color = series_render_color(theme, color_mode, series);
        frame.fill_rectangle(Point::new(8.0, legend_y), Size::new(8.0, 8.0), color);
        frame.fill_text(canvas::Text {
            content: legend_label(&series.display, points),
            position: Point::new(20.0, legend_y + 4.0),
            color,
            size: iced::Pixels(10.0),
            align_x: alignment::Horizontal::Left.into(),
            align_y: alignment::Vertical::Center,
            font: crate::app_fonts::monospace_font(),
            ..canvas::Text::default()
        });
        legend_y += 14.0;
    }
}

pub(in crate::spaghetti::normalized) fn legend_label(
    display: &str,
    points: &[(f32, f64)],
) -> String {
    points
        .last()
        .map(|(_, pct)| format!("{display} {pct:+.2}%"))
        .unwrap_or_else(|| format!("{display} --"))
}

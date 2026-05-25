use iced::Point;
use iced::widget::canvas;

use super::super::super::style::SkeletonPalette;

// ---------------------------------------------------------------------------
// Borders
// ---------------------------------------------------------------------------

pub(in crate::chart_views::skeleton) fn draw_axis_borders(
    frame: &mut canvas::Frame,
    chart_w: f32,
    chart_h: f32,
    funding_h: f32,
    height: f32,
    palette: &SkeletonPalette,
) {
    let stroke = canvas::Stroke::default()
        .with_color(palette.axis)
        .with_width(1.0);
    frame.stroke(
        &canvas::Path::line(Point::new(chart_w, 0.0), Point::new(chart_w, height)),
        stroke,
    );
    frame.stroke(
        &canvas::Path::line(
            Point::new(0.0, chart_h + funding_h),
            Point::new(chart_w, chart_h + funding_h),
        ),
        stroke,
    );

    if funding_h > 0.0 {
        frame.stroke(
            &canvas::Path::line(Point::new(0.0, chart_h), Point::new(chart_w, chart_h)),
            stroke,
        );
    }
}

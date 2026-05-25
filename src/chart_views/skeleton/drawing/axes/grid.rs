use iced::Point;
use iced::widget::canvas;

use super::super::super::style::SkeletonPalette;

// ---------------------------------------------------------------------------
// Grid
// ---------------------------------------------------------------------------

const GRID_LINE_COUNT: usize = 5;
const TIME_AXIS_TICK_COUNT: usize = 5;

pub(in crate::chart_views::skeleton) fn draw_chart_grid(
    frame: &mut canvas::Frame,
    chart_w: f32,
    chart_h: f32,
    palette: &SkeletonPalette,
) {
    let stroke = canvas::Stroke::default()
        .with_color(palette.grid)
        .with_width(1.0);

    for idx in 1..=GRID_LINE_COUNT {
        let y = chart_h * idx as f32 / (GRID_LINE_COUNT + 1) as f32;
        frame.stroke(
            &canvas::Path::line(Point::new(0.0, y), Point::new(chart_w, y)),
            stroke,
        );
    }

    for idx in 1..=TIME_AXIS_TICK_COUNT {
        let x = chart_w * idx as f32 / (TIME_AXIS_TICK_COUNT + 1) as f32;
        frame.stroke(
            &canvas::Path::line(Point::new(x, 0.0), Point::new(x, chart_h)),
            stroke,
        );
    }
}

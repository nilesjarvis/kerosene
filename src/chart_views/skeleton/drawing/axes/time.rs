use iced::widget::canvas;
use iced::{Color, Point, Size};

use super::super::super::style::{Shimmer, SkeletonPalette};

// ---------------------------------------------------------------------------
// Time Axis
// ---------------------------------------------------------------------------

const TIME_AXIS_TICK_COUNT: usize = 5;

pub(in crate::chart_views::skeleton) fn draw_time_axis(
    frame: &mut canvas::Frame,
    chart_w: f32,
    axis_y: f32,
    axis_h: f32,
    palette: &SkeletonPalette,
) {
    draw_time_axis_marks(frame, chart_w, axis_y, axis_h, palette.axis_label, None);
}

pub(in crate::chart_views::skeleton) fn draw_time_axis_shimmer(
    frame: &mut canvas::Frame,
    chart_w: f32,
    axis_y: f32,
    axis_h: f32,
    shimmer: &Shimmer,
) {
    draw_time_axis_marks(
        frame,
        chart_w,
        axis_y,
        axis_h,
        shimmer.color(),
        Some(shimmer),
    );
}

fn draw_time_axis_marks(
    frame: &mut canvas::Frame,
    chart_w: f32,
    axis_y: f32,
    axis_h: f32,
    color: Color,
    shimmer: Option<&Shimmer>,
) {
    let label_h = 5.0;
    for idx in 0..TIME_AXIS_TICK_COUNT {
        let x = chart_w * (idx as f32 + 0.5) / TIME_AXIS_TICK_COUNT as f32;
        let width = 36.0 + (idx % 2) as f32 * 10.0;
        let mark_color = shimmer
            .and_then(|shimmer| shimmer.color_at(x))
            .unwrap_or(color);
        if mark_color.a <= 0.0 {
            continue;
        }
        frame.fill_rectangle(
            Point::new(x - 18.0, axis_y + axis_h * 0.5 - label_h * 0.5),
            Size::new(width, label_h),
            mark_color,
        );
    }
}

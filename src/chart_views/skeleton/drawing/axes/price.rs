use iced::widget::canvas;
use iced::{Color, Point, Size};

use super::super::super::style::{Shimmer, SkeletonPalette};

// ---------------------------------------------------------------------------
// Price Axis
// ---------------------------------------------------------------------------

const PRICE_AXIS_LABEL_COUNT: usize = 6;

pub(in crate::chart_views::skeleton) fn draw_price_axis(
    frame: &mut canvas::Frame,
    width: f32,
    price_axis_w: f32,
    chart_h: f32,
    palette: &SkeletonPalette,
) {
    draw_price_axis_marks(
        frame,
        width,
        price_axis_w,
        chart_h,
        palette.axis_label,
        None,
    );
}

pub(in crate::chart_views::skeleton) fn draw_price_axis_shimmer(
    frame: &mut canvas::Frame,
    width: f32,
    price_axis_w: f32,
    chart_h: f32,
    shimmer: &Shimmer,
) {
    draw_price_axis_marks(
        frame,
        width,
        price_axis_w,
        chart_h,
        shimmer.color(),
        Some(shimmer),
    );
}

fn draw_price_axis_marks(
    frame: &mut canvas::Frame,
    width: f32,
    price_axis_w: f32,
    chart_h: f32,
    color: Color,
    shimmer: Option<&Shimmer>,
) {
    let axis_x = (width - price_axis_w).max(0.0);
    let label_w = (price_axis_w - 18.0).max(22.0);
    let label_h = 5.0;

    for idx in 0..PRICE_AXIS_LABEL_COUNT {
        let y = chart_h * (idx as f32 + 0.5) / PRICE_AXIS_LABEL_COUNT as f32;
        let label_x = axis_x + 10.0;
        let mark_w = label_w * (0.64 + (idx % 3) as f32 * 0.12);
        let mark_color = shimmer
            .and_then(|shimmer| shimmer.color_at(label_x + mark_w * 0.5))
            .unwrap_or(color);
        if mark_color.a <= 0.0 {
            continue;
        }
        frame.fill_rectangle(
            Point::new(label_x, (y - label_h * 0.5).max(0.0)),
            Size::new(mark_w, label_h),
            mark_color,
        );
    }
}

use iced::widget::canvas;
use iced::{Color, Point, Size};

use super::super::sample::{API_SAMPLE_CANDLES, MIN_CANDLE_SPACING, SKELETON_CANDLE_COUNT};
use super::super::style::{Shimmer, SkeletonPalette};

// ---------------------------------------------------------------------------
// Skeleton Candle Drawing
// ---------------------------------------------------------------------------

pub(in crate::chart_views::skeleton) fn draw_skeleton_candles(
    frame: &mut canvas::Frame,
    chart_w: f32,
    chart_h: f32,
    palette: &SkeletonPalette,
) {
    draw_skeleton_candle_marks(
        frame,
        chart_w,
        chart_h,
        palette.candle,
        palette.volume,
        None,
    );
}

pub(in crate::chart_views::skeleton) fn draw_skeleton_candles_shimmer(
    frame: &mut canvas::Frame,
    chart_w: f32,
    chart_h: f32,
    shimmer: &Shimmer,
) {
    draw_skeleton_candle_marks(
        frame,
        chart_w,
        chart_h,
        shimmer.color(),
        shimmer.color(),
        Some(shimmer),
    );
}

fn draw_skeleton_candle_marks(
    frame: &mut canvas::Frame,
    chart_w: f32,
    chart_h: f32,
    candle_color: Color,
    volume_color: Color,
    shimmer: Option<&Shimmer>,
) {
    let volume_h = chart_h * 0.18;
    let price_h = (chart_h - volume_h).max(0.0);
    let visible_count = ((chart_w / MIN_CANDLE_SPACING).floor() as usize)
        .clamp(12, SKELETON_CANDLE_COUNT)
        .min(API_SAMPLE_CANDLES.len());
    let sample_start = API_SAMPLE_CANDLES.len().saturating_sub(visible_count);
    let step = chart_w / visible_count as f32;
    let candle_w = (step * 0.58).clamp(1.0, 9.0);
    let price_pad = (price_h * 0.06).min(12.0);
    let price_draw_h = (price_h - price_pad * 2.0).max(1.0);
    let price_to_y =
        |value: f32| -> f32 { price_pad + (1.0 - value.clamp(0.0, 1.0)) * price_draw_h };

    for idx in 0..visible_count {
        let candle = API_SAMPLE_CANDLES[sample_start + idx];
        let cx = chart_w - step * (visible_count.saturating_sub(idx) as f32 + 0.35);
        let open_y = price_to_y(candle.open);
        let close_y = price_to_y(candle.close);
        let high_y = price_to_y(candle.high);
        let low_y = price_to_y(candle.low);
        let body_top = open_y.min(close_y);
        let body_h = (open_y - close_y).abs().max(2.0);
        let mark_candle_color = shimmer
            .and_then(|shimmer| shimmer.color_at(cx))
            .unwrap_or(candle_color);
        if mark_candle_color.a <= 0.0 {
            continue;
        }

        frame.stroke(
            &canvas::Path::line(Point::new(cx, high_y), Point::new(cx, low_y)),
            canvas::Stroke::default()
                .with_color(mark_candle_color)
                .with_width(1.0),
        );
        frame.fill_rectangle(
            Point::new(cx - candle_w * 0.5, body_top),
            Size::new(candle_w, body_h),
            mark_candle_color,
        );

        let volume_height = (volume_h * (0.12 + candle.volume * 0.76)).max(2.0);
        let mark_volume_color = shimmer
            .and_then(|shimmer| shimmer.color_at(cx))
            .unwrap_or(volume_color);
        frame.fill_rectangle(
            Point::new(cx - candle_w * 0.5, chart_h - volume_height),
            Size::new(candle_w, volume_height),
            mark_volume_color,
        );
    }
}

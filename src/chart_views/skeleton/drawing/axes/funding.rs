use iced::widget::canvas;
use iced::{Color, Point, Size};

use super::super::super::style::{Shimmer, SkeletonPalette};

// ---------------------------------------------------------------------------
// Funding Panel
// ---------------------------------------------------------------------------

pub(in crate::chart_views::skeleton) fn draw_funding_panel(
    frame: &mut canvas::Frame,
    chart_w: f32,
    chart_h: f32,
    funding_h: f32,
    palette: &SkeletonPalette,
) {
    let baseline_y = chart_h + funding_h * 0.52;
    frame.stroke(
        &canvas::Path::line(Point::new(0.0, baseline_y), Point::new(chart_w, baseline_y)),
        canvas::Stroke::default()
            .with_color(palette.grid)
            .with_width(1.0),
    );
    draw_funding_panel_marks(frame, chart_w, chart_h, funding_h, palette.funding, None);
}

pub(in crate::chart_views::skeleton) fn draw_funding_panel_shimmer(
    frame: &mut canvas::Frame,
    chart_w: f32,
    chart_h: f32,
    funding_h: f32,
    shimmer: &Shimmer,
) {
    draw_funding_panel_marks(
        frame,
        chart_w,
        chart_h,
        funding_h,
        shimmer.color(),
        Some(shimmer),
    );
}

fn draw_funding_panel_marks(
    frame: &mut canvas::Frame,
    chart_w: f32,
    chart_h: f32,
    funding_h: f32,
    color: Color,
    shimmer: Option<&Shimmer>,
) {
    let panel_y = chart_h;
    let baseline_y = panel_y + funding_h * 0.52;

    let segments = 24;
    let step = chart_w / segments as f32;
    for idx in 0..segments {
        let x = idx as f32 * step + step * 0.28;
        let height = funding_h * (0.12 + ((idx as f32 * 0.7).sin().abs() * 0.24));
        let bar_w = (step * 0.38).max(2.0);
        let mark_color = shimmer
            .and_then(|shimmer| shimmer.color_at(x + bar_w * 0.5))
            .unwrap_or(color);
        if mark_color.a <= 0.0 {
            continue;
        }
        frame.fill_rectangle(
            Point::new(x, baseline_y - height * 0.5),
            Size::new(bar_w, height),
            mark_color,
        );
    }
}

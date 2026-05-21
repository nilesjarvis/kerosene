use super::PairRatioRenderContext;
use super::format_ratio_value;
use crate::spaghetti::helpers::format_relative_time;
use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point};

// ---------------------------------------------------------------------------
// Pair Ratio Crosshair
// ---------------------------------------------------------------------------

pub(super) fn draw_ratio_crosshair(
    ctx: &PairRatioRenderContext<'_>,
    ratio_hi: f64,
    ratio_range: f64,
) -> canvas::Geometry {
    let mut overlay = canvas::Frame::new(ctx.renderer, ctx.bounds.size());
    if let Some(pos) = ctx.state.cursor_position
        && pos.x < ctx.chart_w
        && pos.y < ctx.chart_h
    {
        let h = canvas::Path::line(Point::new(0.0, pos.y), Point::new(ctx.chart_w, pos.y));
        let v = canvas::Path::line(Point::new(pos.x, 0.0), Point::new(pos.x, ctx.chart_h));
        let stroke = canvas::Stroke::default()
            .with_color(Color {
                a: 0.25,
                ..ctx.theme.palette().text
            })
            .with_width(0.5);
        overlay.stroke(&h, stroke);
        overlay.stroke(&v, stroke);

        let hover_ratio = ratio_hi - (pos.y as f64 / ctx.chart_h as f64) * ratio_range;
        overlay.fill_text(canvas::Text {
            content: format_ratio_value(hover_ratio),
            position: Point::new(ctx.chart_w + 4.0, pos.y),
            color: Color::WHITE,
            size: iced::Pixels(10.0),
            align_x: alignment::Horizontal::Left.into(),
            align_y: alignment::Vertical::Center,
            font: crate::app_fonts::monospace_font(),
            ..canvas::Text::default()
        });

        let cursor_ts = ctx.left_ts + (pos.x as f64 / ctx.time_px_per_ms);
        let delta = ctx.effective_max as f64 - cursor_ts;
        overlay.fill_text(canvas::Text {
            content: format_relative_time(delta),
            position: Point::new(pos.x, ctx.chart_h + 4.0),
            color: Color::WHITE,
            size: iced::Pixels(10.0),
            align_x: alignment::Horizontal::Center.into(),
            align_y: alignment::Vertical::Top,
            font: crate::app_fonts::monospace_font(),
            ..canvas::Text::default()
        });
    }
    overlay.into_geometry()
}

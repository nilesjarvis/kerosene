use iced::widget::canvas;
use iced::{Color, Point, Size, alignment};

use super::fisheye::ChartFisheye;
use crate::annotations::LineStyle;

#[derive(Debug, Clone, Copy)]
pub(super) struct AxisBadgeStyle {
    pub(super) char_width: f32,
    pub(super) padding_width: f32,
    pub(super) height: f32,
    pub(super) text_size: f32,
    pub(super) text_color: Color,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SegmentedHLineStyle {
    pub(super) segment_len: f32,
    pub(super) gap_len: f32,
    pub(super) offset: f32,
    pub(super) color: Color,
    pub(super) width: f32,
}

pub(super) fn stroke_projected_hline(
    frame: &mut canvas::Frame,
    fisheye: ChartFisheye,
    chart_w: f32,
    y: f32,
    color: Color,
    width: f32,
) {
    if chart_w <= 0.0 {
        return;
    }

    // Price-anchored lines (current price, positions, orders) stay sharp:
    // they carry information the trader reads at the edges too.
    fisheye.stroke_projected_line_without_edge_blur(
        frame,
        Point::new(0.0, y),
        Point::new(chart_w, y),
        canvas::Stroke::default()
            .with_color(color)
            .with_width(width),
    );
}

pub(super) fn stroke_projected_segmented_hline_with_offset(
    frame: &mut canvas::Frame,
    fisheye: ChartFisheye,
    chart_w: f32,
    y: f32,
    style: SegmentedHLineStyle,
) {
    if chart_w <= 0.0 || style.segment_len <= 0.0 {
        return;
    }

    let stride = (style.segment_len + style.gap_len).max(style.segment_len);
    let phase = style.offset.rem_euclid(stride);
    let mut x = phase - stride;
    let stroke = canvas::Stroke::default()
        .with_color(style.color)
        .with_width(style.width);
    while x < chart_w {
        let start = x.max(0.0);
        let end = (x + style.segment_len).min(chart_w);
        if end > start {
            fisheye.stroke_projected_line_without_edge_blur(
                frame,
                Point::new(start, y),
                Point::new(end, y),
                stroke,
            );
        }
        x += stride;
    }
}

/// Stroke a line between two source points honoring a [`LineStyle`].
/// Dashed/dotted variants are walked at a fixed pattern along the segment.
pub(super) fn stroke_projected_styled_line(
    frame: &mut canvas::Frame,
    fisheye: ChartFisheye,
    a: Point,
    b: Point,
    color: Color,
    width: f32,
    style: LineStyle,
) {
    match style {
        LineStyle::Solid => {
            fisheye.stroke_projected_line(
                frame,
                a,
                b,
                canvas::Stroke::default()
                    .with_color(color)
                    .with_width(width),
            );
        }
        LineStyle::Dashed => {
            stroke_projected_dash_pattern(frame, fisheye, a, b, color, width, (6.0, 4.0))
        }
        LineStyle::Dotted => {
            stroke_projected_dash_pattern(frame, fisheye, a, b, color, width, (1.5, 3.0))
        }
    }
}

/// Stroke a dashed/dotted line along an arbitrary-angle segment. `pattern` is
/// `(segment_len, gap_len)` in pixels.
fn stroke_projected_dash_pattern(
    frame: &mut canvas::Frame,
    fisheye: ChartFisheye,
    a: Point,
    b: Point,
    color: Color,
    width: f32,
    pattern: (f32, f32),
) {
    let (segment_len, gap_len) = pattern;
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-3 || segment_len <= 0.0 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;
    let stride = (segment_len + gap_len).max(segment_len);
    let stroke = canvas::Stroke::default()
        .with_color(color)
        .with_width(width);
    let mut t = 0.0;
    while t < len {
        let start = t;
        let end = (t + segment_len).min(len);
        fisheye.stroke_projected_line(
            frame,
            Point::new(a.x + ux * start, a.y + uy * start),
            Point::new(a.x + ux * end, a.y + uy * end),
            stroke,
        );
        t += stride;
    }
}

/// Stroke a vertical line at source `x` spanning `0..height`.
pub(super) fn stroke_projected_vline(
    frame: &mut canvas::Frame,
    fisheye: ChartFisheye,
    x: f32,
    height: f32,
    color: Color,
    width: f32,
    style: LineStyle,
) {
    if height <= 0.0 {
        return;
    }
    stroke_projected_styled_line(
        frame,
        fisheye,
        Point::new(x, 0.0),
        Point::new(x, height),
        color,
        width,
        style,
    );
}

pub(super) fn fill_right_axis_badge(
    frame: &mut canvas::Frame,
    chart_w: f32,
    center_y: f32,
    label: String,
    background: Color,
    style: AxisBadgeStyle,
) {
    let badge_w = label.len() as f32 * style.char_width + style.padding_width;
    let badge_x = chart_w + 1.0;
    let badge_y = center_y - style.height * 0.5;
    let background = Color {
        a: 1.0,
        ..background
    };

    frame.fill_rectangle(
        Point::new(badge_x, badge_y),
        Size::new(badge_w, style.height),
        background,
    );
    frame.fill_text(canvas::Text {
        content: label,
        position: Point::new(badge_x + 4.0, center_y),
        color: style.text_color,
        size: iced::Pixels(style.text_size),
        align_x: alignment::Horizontal::Left.into(),
        align_y: alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
}

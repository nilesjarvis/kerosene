use crate::chart::drawing::SegmentedHLineStyle;
pub(super) use crate::chart::segmented_curve::append_segmented_quadratic_curve;

use iced::Point;
use iced::widget::canvas;

// ---------------------------------------------------------------------------
// Segmented Geometry
// ---------------------------------------------------------------------------

pub(super) fn stroke_segmented_hline_range(
    frame: &mut canvas::Frame,
    start_x: f32,
    end_x: f32,
    y: f32,
    style: SegmentedHLineStyle,
) {
    if end_x <= start_x
        || style.segment_len <= 0.0
        || !start_x.is_finite()
        || !end_x.is_finite()
        || !y.is_finite()
        || !style.segment_len.is_finite()
        || !style.gap_len.is_finite()
    {
        return;
    }

    let stride = style.segment_len + style.gap_len.max(0.0);
    if stride <= 0.0 || !stride.is_finite() {
        return;
    }

    let phase = if style.offset.is_finite() {
        style.offset.rem_euclid(stride)
    } else {
        0.0
    };

    let mut has_segment = false;
    let line = canvas::Path::new(|path| {
        let mut x = phase - stride;
        while x < end_x {
            let start = x.max(start_x);
            let end = (x + style.segment_len).min(end_x);
            if end > start {
                path.move_to(Point::new(start, y));
                path.line_to(Point::new(end, y));
                has_segment = true;
            }
            x += stride;
        }
    });

    if has_segment {
        frame.stroke(&line, order_solid_stroke(&style));
    }
}

pub(super) fn order_solid_stroke(style: &SegmentedHLineStyle) -> canvas::Stroke<'static> {
    canvas::Stroke::default()
        .with_color(style.color)
        .with_width(style.width)
}

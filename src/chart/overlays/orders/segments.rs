use crate::chart::drawing::SegmentedHLineStyle;
use crate::chart::fisheye::ChartFisheye;
use crate::chart::segmented_curve::{quadratic_point, valid_point};

use iced::Point;
use iced::widget::canvas;

// ---------------------------------------------------------------------------
// Segmented Geometry
// ---------------------------------------------------------------------------

pub(super) fn stroke_segmented_hline_range(
    frame: &mut canvas::Frame,
    fisheye: ChartFisheye,
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

    if fisheye.is_enabled() {
        let mut x = phase - stride;
        let stroke = order_solid_stroke(&style);
        while x < end_x {
            let start = x.max(start_x);
            let end = (x + style.segment_len).min(end_x);
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
    } else {
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
}

pub(super) fn stroke_segmented_quadratic_curve(
    frame: &mut canvas::Frame,
    fisheye: ChartFisheye,
    start: Point,
    control: Point,
    end: Point,
    style: &SegmentedHLineStyle,
) {
    const SAMPLES: usize = 18;

    if !valid_point(start)
        || !valid_point(control)
        || !valid_point(end)
        || style.segment_len <= 0.0
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
    let mut next_dash_start = phase - stride;
    while next_dash_start + style.segment_len <= 0.0 {
        next_dash_start += stride;
    }

    let stroke = order_solid_stroke(style);
    let mut previous = start;
    let mut previous_distance = 0.0;
    for sample in 1..=SAMPLES {
        let t = sample as f32 / SAMPLES as f32;
        let current = quadratic_point(start, control, end, t);
        let segment_len = distance(previous, current);
        if segment_len > 0.0 && segment_len.is_finite() {
            let segment_start_distance = previous_distance;
            let segment_end_distance = previous_distance + segment_len;

            while next_dash_start < segment_end_distance {
                let dash_start = next_dash_start.max(segment_start_distance);
                let dash_end = (next_dash_start + style.segment_len).min(segment_end_distance);
                if dash_end > dash_start {
                    let start_t = (dash_start - segment_start_distance) / segment_len;
                    let end_t = (dash_end - segment_start_distance) / segment_len;
                    fisheye.stroke_projected_line_without_edge_blur(
                        frame,
                        lerp_point(previous, current, start_t),
                        lerp_point(previous, current, end_t),
                        stroke,
                    );
                }
                next_dash_start += stride;
            }

            previous_distance = segment_end_distance;
        }
        previous = current;
    }
}

pub(super) fn order_solid_stroke(style: &SegmentedHLineStyle) -> canvas::Stroke<'static> {
    canvas::Stroke::default()
        .with_color(style.color)
        .with_width(style.width)
}

fn lerp_point(start: Point, end: Point, t: f32) -> Point {
    Point::new(
        start.x + (end.x - start.x) * t,
        start.y + (end.y - start.y) * t,
    )
}

fn distance(start: Point, end: Point) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    (dx * dx + dy * dy).sqrt()
}

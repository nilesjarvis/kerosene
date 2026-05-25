use crate::chart::drawing::SegmentedHLineStyle;

use iced::Point;
use iced::widget::canvas;

// ---------------------------------------------------------------------------
// Segmented Quadratic Curves
// ---------------------------------------------------------------------------

pub(in crate::chart) fn append_segmented_quadratic_curve(
    path: &mut canvas::path::Builder,
    start: Point,
    control: Point,
    end: Point,
    style: &SegmentedHLineStyle,
) -> bool {
    const SAMPLES: usize = 18;

    if !valid_point(start)
        || !valid_point(control)
        || !valid_point(end)
        || style.segment_len <= 0.0
        || !style.segment_len.is_finite()
        || !style.gap_len.is_finite()
    {
        return false;
    }

    let stride = style.segment_len + style.gap_len.max(0.0);
    if stride <= 0.0 || !stride.is_finite() {
        return false;
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

    let mut previous = start;
    let mut previous_distance = 0.0;
    let mut has_segment = false;
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
                    path.move_to(lerp_point(previous, current, start_t));
                    path.line_to(lerp_point(previous, current, end_t));
                    has_segment = true;
                }
                next_dash_start += stride;
            }

            previous_distance = segment_end_distance;
        }
        previous = current;
    }
    has_segment
}

pub(in crate::chart) fn quadratic_point(start: Point, control: Point, end: Point, t: f32) -> Point {
    let inv_t = 1.0 - t;
    let start_weight = inv_t * inv_t;
    let control_weight = 2.0 * inv_t * t;
    let end_weight = t * t;
    Point::new(
        start.x * start_weight + control.x * control_weight + end.x * end_weight,
        start.y * start_weight + control.y * control_weight + end.y * end_weight,
    )
}

pub(in crate::chart) fn valid_point(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
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

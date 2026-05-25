use iced::widget::canvas;
use iced::{Color, Point};

// ---------------------------------------------------------------------------
// Segmented Label Connectors
// ---------------------------------------------------------------------------

const SERIES_LABEL_DASH: f32 = 1.0;
const SERIES_LABEL_GAP_DASH: f32 = 4.0;

pub(super) fn stroke_segmented_hline_range(
    frame: &mut canvas::Frame,
    start_x: f32,
    end_x: f32,
    y: f32,
    color: Color,
) {
    if end_x <= start_x || !start_x.is_finite() || !end_x.is_finite() || !y.is_finite() {
        return;
    }

    let mut has_segment = false;
    let line = canvas::Path::new(|path| {
        let mut x = start_x;
        while x < end_x {
            let end = (x + SERIES_LABEL_DASH).min(end_x);
            if end > x {
                path.move_to(Point::new(x, y));
                path.line_to(Point::new(end, y));
                has_segment = true;
            }
            x += SERIES_LABEL_DASH + SERIES_LABEL_GAP_DASH;
        }
    });

    if has_segment {
        frame.stroke(
            &line,
            canvas::Stroke::default().with_color(color).with_width(1.0),
        );
    }
}

pub(super) fn stroke_segmented_label_connector(
    frame: &mut canvas::Frame,
    start: Point,
    control: Point,
    end: Point,
    color: Color,
) {
    let mut has_segment = false;
    let connector = canvas::Path::new(|path| {
        has_segment = append_segmented_quadratic_curve(path, start, control, end);
    });

    if has_segment {
        frame.stroke(
            &connector,
            canvas::Stroke::default().with_color(color).with_width(1.0),
        );
    }
}

fn append_segmented_quadratic_curve(
    path: &mut canvas::path::Builder,
    start: Point,
    control: Point,
    end: Point,
) -> bool {
    const SAMPLES: usize = 14;

    if !valid_point(start) || !valid_point(control) || !valid_point(end) {
        return false;
    }

    let stride = SERIES_LABEL_DASH + SERIES_LABEL_GAP_DASH;
    let mut next_dash_start = 0.0;
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
                let dash_end = (next_dash_start + SERIES_LABEL_DASH).min(segment_end_distance);
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

fn quadratic_point(start: Point, control: Point, end: Point, t: f32) -> Point {
    let inv_t = 1.0 - t;
    let start_weight = inv_t * inv_t;
    let control_weight = 2.0 * inv_t * t;
    let end_weight = t * t;
    Point::new(
        start.x * start_weight + control.x * control_weight + end.x * end_weight,
        start.y * start_weight + control.y * control_weight + end.y * end_weight,
    )
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

fn valid_point(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

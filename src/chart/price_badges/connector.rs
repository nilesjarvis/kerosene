use super::{RightAxisBadgeKind, RightAxisBadgeLayout};
use crate::chart::drawing::SegmentedHLineStyle;
use crate::chart::fisheye::ChartFisheye;
use crate::chart::segmented_curve::{
    append_segmented_quadratic_curve, quadratic_point, valid_point,
};

use iced::widget::canvas;
use iced::{Color, Point};

// ---------------------------------------------------------------------------
// Right Axis Badge Connectors
// ---------------------------------------------------------------------------

const RIGHT_AXIS_BADGE_CONNECTOR_SPAN: f32 = 18.0;
const RIGHT_AXIS_BADGE_CONNECTOR_SHIFT_EPSILON: f32 = 1.0;

#[derive(Debug, Clone, Copy)]
pub(in crate::chart) enum RightAxisBadgeConnectorStyle {
    Solid { color: Color, width: f32 },
    Segmented { style: SegmentedHLineStyle },
}

pub(in crate::chart) fn right_axis_line_end_x(
    layout: &RightAxisBadgeLayout,
    kind: RightAxisBadgeKind,
    chart_w: f32,
) -> f32 {
    if chart_w <= 0.0 || !chart_w.is_finite() {
        return 0.0;
    }

    if let Some(position) = layout.position(kind)
        && right_axis_badge_is_shifted(position.source_y, position.badge_y)
    {
        return (chart_w - RIGHT_AXIS_BADGE_CONNECTOR_SPAN).max(0.0);
    }

    chart_w
}

pub(super) fn right_axis_badge_connector_points(
    layout: &RightAxisBadgeLayout,
    kind: RightAxisBadgeKind,
    raw_source_y: f32,
    chart_w: f32,
    fisheye: ChartFisheye,
) -> Option<(Point, Point, Point)> {
    let position = layout.position(kind)?;
    if !right_axis_badge_is_shifted(position.source_y, position.badge_y) {
        return None;
    }

    let start_x = right_axis_line_end_x(layout, kind, chart_w);
    let start = fisheye.project(Point::new(start_x, raw_source_y));
    let end = Point::new(chart_w + 1.0, position.badge_y);
    let control = Point::new(start.x + (end.x - start.x) * 0.55, start.y);
    Some((start, control, end))
}

pub(super) fn stroke_right_axis_connector(
    frame: &mut canvas::Frame,
    start: Point,
    control: Point,
    end: Point,
    style: RightAxisBadgeConnectorStyle,
) {
    match style {
        RightAxisBadgeConnectorStyle::Solid { color, width } => {
            let Some(path) = solid_quadratic_path(start, control, end) else {
                return;
            };
            frame.stroke(
                &path,
                canvas::Stroke::default()
                    .with_color(color)
                    .with_width(width)
                    .with_line_cap(canvas::LineCap::Round),
            );
        }
        RightAxisBadgeConnectorStyle::Segmented { style } => {
            let mut has_segment = false;
            let connector = canvas::Path::new(|path| {
                has_segment = append_segmented_quadratic_curve(path, start, control, end, &style);
            });
            if has_segment {
                frame.stroke(
                    &connector,
                    canvas::Stroke::default()
                        .with_color(style.color)
                        .with_width(style.width)
                        .with_line_cap(canvas::LineCap::Round),
                );
            }
        }
    }
}

fn solid_quadratic_path(start: Point, control: Point, end: Point) -> Option<canvas::Path> {
    const SAMPLES: usize = 14;

    if !valid_point(start) || !valid_point(control) || !valid_point(end) {
        return None;
    }

    Some(canvas::Path::new(|path| {
        path.move_to(start);
        for sample in 1..=SAMPLES {
            let t = sample as f32 / SAMPLES as f32;
            path.line_to(quadratic_point(start, control, end, t));
        }
    }))
}

fn right_axis_badge_is_shifted(source_y: f32, badge_y: f32) -> bool {
    (badge_y - source_y).abs() >= RIGHT_AXIS_BADGE_CONNECTOR_SHIFT_EPSILON
}

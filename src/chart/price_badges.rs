mod connector;
mod layout;

pub(super) use self::layout::{
    RIGHT_AXIS_PRIMARY_BADGE_HEIGHT, RIGHT_AXIS_SECONDARY_BADGE_HEIGHT, RightAxisBadgeKind,
    RightAxisBadgeLayout,
};
pub(super) use connector::{RightAxisBadgeConnectorStyle, right_axis_line_end_x};

use self::connector::{right_axis_badge_connector_points, stroke_right_axis_connector};
use super::drawing::{AxisBadgeStyle, fill_right_axis_badge};
use iced::Color;
use iced::widget::canvas;

// ---------------------------------------------------------------------------
// Right Axis Price Badge Rendering
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_stacked_right_axis_badge(
    frame: &mut canvas::Frame,
    layout: &RightAxisBadgeLayout,
    kind: RightAxisBadgeKind,
    chart_w: f32,
    source_y: f32,
    label: String,
    background: Color,
    badge_style: AxisBadgeStyle,
    connector_style: RightAxisBadgeConnectorStyle,
) {
    let badge_y = layout
        .position(kind)
        .map_or(source_y, |position| position.badge_y);
    if let Some((start, control, end)) =
        right_axis_badge_connector_points(layout, kind, source_y, chart_w, badge_y)
    {
        stroke_right_axis_connector(frame, start, control, end, connector_style);
    }

    fill_right_axis_badge(frame, chart_w, badge_y, label, background, badge_style);
}

#[cfg(test)]
mod tests;

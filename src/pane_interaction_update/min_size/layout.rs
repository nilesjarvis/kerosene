use crate::pane_state::PaneKind;

use iced::widget::pane_grid;

const ORDER_ENTRY_MIN_WIDTH: f32 = 300.0;
const ORDER_ENTRY_MIN_HEIGHT: f32 = 360.0;

// ---------------------------------------------------------------------------
// Pane Minimum Layout
// ---------------------------------------------------------------------------

pub(super) fn split_node(
    node: &pane_grid::Node,
    split: pane_grid::Split,
) -> Option<(pane_grid::Axis, &pane_grid::Node, &pane_grid::Node)> {
    match node {
        pane_grid::Node::Split { id, axis, a, b, .. } => {
            if *id == split {
                Some((*axis, a, b))
            } else {
                split_node(a, split).or_else(|| split_node(b, split))
            }
        }
        pane_grid::Node::Pane(_) => None,
    }
}

pub(super) fn subtree_contains_order_entry(
    node: &pane_grid::Node,
    panes: &pane_grid::State<PaneKind>,
) -> bool {
    match node {
        pane_grid::Node::Split { a, b, .. } => {
            subtree_contains_order_entry(a, panes) || subtree_contains_order_entry(b, panes)
        }
        pane_grid::Node::Pane(pane) => {
            matches!(panes.get(*pane), Some(PaneKind::OrderEntry))
        }
    }
}

pub(super) fn subtree_min_length(
    node: &pane_grid::Node,
    measured_axis: pane_grid::Axis,
    panes: &pane_grid::State<PaneKind>,
    base_min_size: f32,
    pane_border_thickness: f32,
) -> f32 {
    match node {
        pane_grid::Node::Split { axis, a, b, .. } => {
            let min_a = subtree_min_length(
                a,
                measured_axis,
                panes,
                base_min_size,
                pane_border_thickness,
            );
            let min_b = subtree_min_length(
                b,
                measured_axis,
                panes,
                base_min_size,
                pane_border_thickness,
            );

            if *axis == measured_axis {
                min_a + min_b + pane_border_thickness
            } else {
                min_a.max(min_b)
            }
        }
        pane_grid::Node::Pane(pane) => panes
            .get(*pane)
            .map(|kind| pane_min_length(kind, measured_axis, base_min_size))
            .unwrap_or(base_min_size),
    }
}

fn pane_min_length(kind: &PaneKind, axis: pane_grid::Axis, base_min_size: f32) -> f32 {
    match (kind, axis) {
        (PaneKind::OrderEntry, pane_grid::Axis::Horizontal) => ORDER_ENTRY_MIN_HEIGHT,
        (PaneKind::OrderEntry, pane_grid::Axis::Vertical) => ORDER_ENTRY_MIN_WIDTH,
        _ => base_min_size,
    }
}

pub(in crate::pane_interaction_update) fn clamp_split_ratio(
    ratio: f32,
    axis_length: f32,
    min_a: f32,
    min_b: f32,
    order_entry_in_a: bool,
    order_entry_in_b: bool,
    pane_border_thickness: f32,
) -> f32 {
    if !ratio.is_finite() {
        return 0.5;
    }

    if axis_length <= 0.0 || !axis_length.is_finite() {
        return ratio.clamp(0.0, 1.0);
    }

    let raw_a = (axis_length * ratio - pane_border_thickness / 2.0).round();
    let max_a = axis_length - min_b - pane_border_thickness;
    let target_a = if max_a >= min_a {
        raw_a.clamp(min_a, max_a)
    } else if order_entry_in_a && !order_entry_in_b {
        min_a.min((axis_length - pane_border_thickness).max(0.0))
    } else if order_entry_in_b && !order_entry_in_a {
        (axis_length - min_b - pane_border_thickness).max(0.0)
    } else {
        raw_a.clamp(0.0, (axis_length - pane_border_thickness).max(0.0))
    };

    ((target_a + pane_border_thickness / 2.0) / axis_length).clamp(0.0, 1.0)
}

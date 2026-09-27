use super::plot::SnapshotPlot;
use crate::chart::TradeMarker;
use iced::widget::canvas;
use iced::{Color, Point, Theme};

const SNAPSHOT_MARKER_RADIUS: f32 = 2.8;
const SNAPSHOT_MARKER_GROUP_RADIUS: f32 = 3.8;
const SNAPSHOT_MARKER_CHART_GAP: f32 = 2.0;
const SNAPSHOT_MARKER_GROUP_GAP: f32 = 2.0;
const SNAPSHOT_MARKER_OFFSET: f32 = SNAPSHOT_MARKER_GROUP_RADIUS + SNAPSHOT_MARKER_CHART_GAP;
const SNAPSHOT_MARKER_GROUP_DISTANCE: f32 =
    SNAPSHOT_MARKER_GROUP_RADIUS * 2.0 + SNAPSHOT_MARKER_GROUP_GAP;

pub(super) fn draw_markers(
    frame: &mut canvas::Frame,
    theme: &Theme,
    plot: SnapshotPlot,
    markers: &[TradeMarker],
) {
    let mut marker_layouts = marker_group_layouts(plot, markers);
    marker_layouts.sort_by(|a, b| a.center.x.total_cmp(&b.center.x));
    let outline_color = Color {
        a: 0.72,
        ..theme.extended_palette().background.strong.color
    };

    for layout in marker_layouts {
        let marker_color = Color {
            a: 0.9,
            ..if layout.is_buy {
                theme.palette().success
            } else {
                theme.palette().danger
            }
        };
        let dot = canvas::Path::circle(layout.center, layout.radius);
        frame.fill(&dot, marker_color);
        frame.stroke(
            &dot,
            canvas::Stroke::default()
                .with_color(outline_color)
                .with_width(0.75),
        );
    }
}

#[derive(Debug, Clone, Copy)]
struct SnapshotMarkerLayout {
    center: Point,
    radius: f32,
    is_buy: bool,
}

fn marker_group_layouts(plot: SnapshotPlot, markers: &[TradeMarker]) -> Vec<SnapshotMarkerLayout> {
    let mut buys: Vec<_> = markers
        .iter()
        .filter(|marker| marker.is_buy)
        .map(|marker| plot.x_for_time(marker.time_ms))
        .collect();
    let mut sells: Vec<_> = markers
        .iter()
        .filter(|marker| !marker.is_buy)
        .map(|marker| plot.x_for_time(marker.time_ms))
        .collect();
    buys.sort_by(|a, b| a.total_cmp(b));
    sells.sort_by(|a, b| a.total_cmp(b));

    let mut layouts = Vec::with_capacity(markers.len());
    push_marker_side_layouts(plot, &sells, false, &mut layouts);
    push_marker_side_layouts(plot, &buys, true, &mut layouts);
    layouts
}

fn push_marker_side_layouts(
    plot: SnapshotPlot,
    marker_xs: &[f32],
    is_buy: bool,
    layouts: &mut Vec<SnapshotMarkerLayout>,
) {
    let Some((&first_x, rest)) = marker_xs.split_first() else {
        return;
    };

    let y = if is_buy {
        plot.top + plot.height + SNAPSHOT_MARKER_OFFSET
    } else {
        plot.top - SNAPSHOT_MARKER_OFFSET
    };
    let mut group_sum = first_x;
    let mut group_count = 1_usize;
    let mut group_last_x = first_x;

    for &x in rest {
        if x - group_last_x > SNAPSHOT_MARKER_GROUP_DISTANCE {
            push_marker_group(layouts, group_sum, group_count, y, is_buy);
            group_sum = x;
            group_count = 1;
        } else {
            group_sum += x;
            group_count += 1;
        }
        group_last_x = x;
    }

    push_marker_group(layouts, group_sum, group_count, y, is_buy);
}

fn push_marker_group(
    layouts: &mut Vec<SnapshotMarkerLayout>,
    group_sum: f32,
    group_count: usize,
    y: f32,
    is_buy: bool,
) {
    let x = group_sum / group_count.max(1) as f32;
    let radius = if group_count > 1 {
        SNAPSHOT_MARKER_GROUP_RADIUS
    } else {
        SNAPSHOT_MARKER_RADIUS
    };
    layouts.push(SnapshotMarkerLayout {
        center: Point::new(x, y),
        radius,
        is_buy,
    });
}

#[cfg(test)]
mod tests;

use super::plot::SnapshotPlot;
use crate::api::Candle;
use crate::chart::TradeMarker;
use iced::widget::canvas;
use iced::{Color, Point, Theme};
use std::collections::BTreeMap;

mod tooltip;

const DOT_RADIUS: f32 = 3.2;
const GROUP_RADIUS: f32 = 4.2;
const WICK_GAP: f32 = 3.0;
const HIT_PADDING: f32 = 5.0;

pub(super) struct MarkerGroup {
    center: Point,
    is_buy: bool,
    count: usize,
    first_ms: u64,
    last_ms: u64,
    size: f64,
    average_price: f64,
}

impl MarkerGroup {
    fn radius(&self) -> f32 {
        if self.count > 1 {
            GROUP_RADIUS
        } else {
            DOT_RADIUS
        }
    }

    fn color(&self, theme: &Theme) -> Color {
        let palette = theme.palette();
        let side = if self.is_buy {
            palette.success
        } else {
            palette.danger
        };
        iced::theme::palette::mix(side, palette.text, 0.18)
    }
}

/// One dot per candle and side. Fills in gaps keep their own time and price.
fn marker_groups(
    plot: SnapshotPlot,
    candles: &[Candle],
    markers: &[TradeMarker],
) -> Vec<MarkerGroup> {
    let mut groups = BTreeMap::new();
    for marker in markers {
        let next = candles.partition_point(|candle| candle.open_time <= marker.time_ms);
        let candle = candles[..next]
            .last()
            .filter(|candle| marker.time_ms <= candle.close_time);
        let time = candle.map_or(marker.time_ms, |candle| candle.open_time);
        let price = candle.map_or(marker.price, |candle| {
            if marker.is_buy {
                candle.low
            } else {
                candle.high
            }
        });
        let y = plot.y_for_price(price);
        let group = groups
            .entry((time, marker.is_buy))
            .or_insert_with(|| MarkerGroup {
                center: Point::new(plot.x_for_time(time), y),
                is_buy: marker.is_buy,
                count: 0,
                first_ms: marker.time_ms,
                last_ms: marker.time_ms,
                size: 0.0,
                average_price: marker.price,
            });
        group.count += 1;
        group.last_ms = marker.time_ms;
        group.size += marker.size;
        group.average_price += (marker.price - group.average_price) * (marker.size / group.size);
        // Same-time fills in a candle gap can have different execution prices.
        group.center.y = if marker.is_buy {
            group.center.y.max(y)
        } else {
            group.center.y.min(y)
        };
    }

    groups
        .into_values()
        .map(|mut group| {
            let offset = group.radius() + WICK_GAP;
            group.center.y += if group.is_buy { offset } else { -offset };
            group
        })
        .collect()
}

fn hit_test(groups: &[MarkerGroup], cursor: Point) -> Option<usize> {
    groups
        .iter()
        .enumerate()
        .filter_map(|(i, group)| {
            let distance = cursor.distance(group.center);
            (distance <= group.radius() + HIT_PADDING).then_some((i, distance))
        })
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(i, _)| i)
}

pub(super) fn draw_markers(
    frame: &mut canvas::Frame,
    theme: &Theme,
    plot: SnapshotPlot,
    candles: &[Candle],
    markers: &[TradeMarker],
    cursor: Option<Point>,
) -> Option<MarkerGroup> {
    let mut groups = marker_groups(plot, candles, markers);
    let hovered = cursor.and_then(|point| hit_test(&groups, point));
    for group in &groups {
        let dot = canvas::Path::circle(group.center, group.radius());
        frame.fill(&dot, group.color(theme));
        frame.stroke(
            &dot,
            canvas::Stroke::default()
                .with_color(theme.palette().background)
                .with_width(1.0),
        );
    }
    hovered.map(|index| {
        let group = groups.swap_remove(index);
        let ring = canvas::Path::circle(group.center, group.radius() + 2.0);
        frame.stroke(
            &ring,
            canvas::Stroke::default()
                .with_color(group.color(theme))
                .with_width(1.2),
        );
        group
    })
}

#[cfg(test)]
mod tests;

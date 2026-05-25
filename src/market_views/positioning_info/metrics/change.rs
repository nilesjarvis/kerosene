use super::live::positioning_live_change_usd;
use crate::config;
use crate::helpers::finite_value;
use crate::hyperdash_api::PerpDeltaEntry;
use crate::positioning_state::PositioningInfoChangeSortField;

use iced::{Color, Theme};

// ---------------------------------------------------------------------------
// Positioning Change Metrics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(in crate::market_views::positioning_info) struct PositioningChangeSideTotals {
    pub(in crate::market_views::positioning_info) long_delta: f64,
    pub(in crate::market_views::positioning_info) short_delta: f64,
}

pub(in crate::market_views::positioning_info) fn positioning_previous_change_size(
    entry: &PerpDeltaEntry,
) -> Option<f64> {
    let previous = entry.current - entry.delta;
    finite_value(previous)
}

pub(in crate::market_views::positioning_info) fn positioning_change_side_delta_totals(
    deltas: &[PerpDeltaEntry],
) -> PositioningChangeSideTotals {
    let mut totals = PositioningChangeSideTotals {
        long_delta: 0.0,
        short_delta: 0.0,
    };

    for entry in deltas {
        if !entry.current.is_finite() {
            continue;
        }
        let Some(previous) = positioning_previous_change_size(entry) else {
            continue;
        };

        totals.long_delta += entry.current.max(0.0) - previous.max(0.0);
        totals.short_delta += (-entry.current).max(0.0) - (-previous).max(0.0);
    }

    totals
}

pub(in crate::market_views::positioning_info) fn positioning_side_delta_color(
    value: f64,
    is_long: bool,
    theme: &Theme,
) -> Color {
    if value == 0.0 || !value.is_finite() {
        return theme.palette().text;
    }
    match (is_long, value > 0.0) {
        (true, true) | (false, false) => theme.palette().success,
        (true, false) | (false, true) => theme.palette().danger,
    }
}

pub(in crate::market_views::positioning_info) fn sorted_change_rows(
    deltas: &[PerpDeltaEntry],
    sort_field: PositioningInfoChangeSortField,
    sort_direction: config::SortDirection,
    live_mark: Option<f64>,
) -> Vec<&PerpDeltaEntry> {
    let mut rows: Vec<&PerpDeltaEntry> = deltas.iter().collect();
    rows.sort_by(|a, b| {
        let ordering = match sort_field {
            PositioningInfoChangeSortField::Trader => a.address.cmp(&b.address),
            PositioningInfoChangeSortField::Previous => optional_number_cmp_directional(
                positioning_previous_change_size(a),
                positioning_previous_change_size(b),
                sort_direction,
            ),
            PositioningInfoChangeSortField::Current => optional_number_cmp_directional(
                finite_number(a.current),
                finite_number(b.current),
                sort_direction,
            ),
            PositioningInfoChangeSortField::Change => optional_number_cmp_directional(
                finite_number(a.delta.abs()),
                finite_number(b.delta.abs()),
                sort_direction,
            ),
            PositioningInfoChangeSortField::CurrentUsd => optional_number_cmp_directional(
                positioning_live_change_usd(a.current, live_mark),
                positioning_live_change_usd(b.current, live_mark),
                sort_direction,
            ),
            PositioningInfoChangeSortField::ChangeUsd => optional_number_cmp_directional(
                positioning_live_change_usd(a.delta, live_mark).map(f64::abs),
                positioning_live_change_usd(b.delta, live_mark).map(f64::abs),
                sort_direction,
            ),
        };
        let ordering = if sort_field == PositioningInfoChangeSortField::Trader
            && sort_direction == config::SortDirection::Descending
        {
            ordering.reverse()
        } else {
            ordering
        };
        ordering.then_with(|| a.address.cmp(&b.address))
    });
    rows
}

fn optional_number_cmp_directional(
    a: Option<f64>,
    b: Option<f64>,
    direction: config::SortDirection,
) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => {
            let ordering = a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal);
            if direction == config::SortDirection::Descending {
                ordering.reverse()
            } else {
                ordering
            }
        }
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn finite_number(value: f64) -> Option<f64> {
    finite_value(value)
}

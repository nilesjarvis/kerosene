use crate::helpers::finite_value;
use crate::hyperdash_api::PerpDeltaEntry;

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

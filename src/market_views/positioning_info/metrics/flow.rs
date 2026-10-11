use super::change::positioning_previous_change_size;
use super::live::positioning_live_change_usd;
use crate::helpers::positive_finite_value;
use crate::hyperdash_api::PerpDeltaEntry;

// ---------------------------------------------------------------------------
// Positioning Change Flow Rows
//
// The Change tab visualizes how each trader's position size moved over the
// selected timeframe as a diverging horizontal bar. To keep the canvas program
// pure and cheap, the heavy lifting (USD conversion, classification, sorting,
// scaling) happens here against the raw deltas.
// ---------------------------------------------------------------------------

/// How a trader's position moved over the timeframe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::market_views::positioning_info) enum PositioningFlowKind {
    /// Grew an existing position (or opened from flat) on the same side.
    Add,
    /// Shrank a position toward zero without crossing it.
    Cut,
    /// Crossed zero: short -> long or long -> short.
    Flip,
}

impl PositioningFlowKind {
    pub(in crate::market_views::positioning_info) fn label(self) -> &'static str {
        match self {
            Self::Add => "Add",
            Self::Cut => "Cut",
            Self::Flip => "Flip",
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::market_views::positioning_info) struct PositioningFlowRow {
    pub(in crate::market_views::positioning_info) address: String,
    /// Signed previous exposure valued at the live mark, in USD, when derivable.
    pub(in crate::market_views::positioning_info) previous_usd: Option<f64>,
    /// Signed change valued at the live mark, in USD (+ = more long).
    pub(in crate::market_views::positioning_info) delta_usd: f64,
    /// Signed current exposure valued at the live mark, in USD.
    pub(in crate::market_views::positioning_info) current_usd: Option<f64>,
    pub(in crate::market_views::positioning_info) kind: PositioningFlowKind,
}

impl PositioningFlowRow {
    /// USD magnitude used for bar scaling and sorting.
    pub(in crate::market_views::positioning_info) fn magnitude(&self) -> f64 {
        self.delta_usd.abs()
    }
}

#[derive(Debug, Clone)]
pub(in crate::market_views::positioning_info) struct PositioningFlowData {
    pub(in crate::market_views::positioning_info) rows: Vec<PositioningFlowRow>,
    /// Largest row magnitude, used to scale bars. Always finite and > 0 when
    /// there is at least one row with a measurable move.
    pub(in crate::market_views::positioning_info) max_magnitude: f64,
    /// Aggregate long-ward delta in USD.
    pub(in crate::market_views::positioning_info) long_flow: f64,
    /// Aggregate short-ward delta in USD.
    pub(in crate::market_views::positioning_info) short_flow: f64,
    /// False when there is no valid live mark to value the changes in USD.
    pub(in crate::market_views::positioning_info) usd_available: bool,
}

fn classify_flow(previous: Option<f64>, current: f64, delta: f64) -> PositioningFlowKind {
    match previous {
        Some(previous)
            if previous != 0.0 && current != 0.0 && previous.signum() != current.signum() =>
        {
            PositioningFlowKind::Flip
        }
        Some(previous) if current.abs() < previous.abs() => PositioningFlowKind::Cut,
        // Opening from flat, or growing on the same side.
        _ => {
            if delta == 0.0 {
                // No measurable move; treat tiny reductions as cuts, else add.
                PositioningFlowKind::Add
            } else if current.abs() >= previous.map(f64::abs).unwrap_or(0.0) {
                PositioningFlowKind::Add
            } else {
                PositioningFlowKind::Cut
            }
        }
    }
}

pub(in crate::market_views::positioning_info) fn positioning_flow_data(
    deltas: &[PerpDeltaEntry],
    live_mark: Option<f64>,
    limit: usize,
) -> PositioningFlowData {
    let live_mark = live_mark.and_then(positive_finite_value);
    let mut rows: Vec<PositioningFlowRow> = Vec::with_capacity(deltas.len());
    let mut long_flow = 0.0;
    let mut short_flow = 0.0;

    for entry in deltas {
        if !entry.current.is_finite() || !entry.delta.is_finite() {
            continue;
        }
        let previous_size = positioning_previous_change_size(entry);
        let Some(delta_usd) = positioning_live_change_usd(entry.delta, live_mark) else {
            continue;
        };
        let current_usd = positioning_live_change_usd(entry.current, live_mark);
        let previous_usd =
            previous_size.and_then(|value| positioning_live_change_usd(value, live_mark));
        let kind = classify_flow(previous_size, entry.current, entry.delta);

        if delta_usd > 0.0 {
            long_flow += delta_usd;
        } else if delta_usd < 0.0 {
            short_flow += delta_usd.abs();
        }

        rows.push(PositioningFlowRow {
            address: entry.address.clone(),
            previous_usd,
            delta_usd,
            current_usd,
            kind,
        });
    }

    rows.sort_by(|a, b| {
        b.magnitude()
            .partial_cmp(&a.magnitude())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.address.cmp(&b.address))
    });
    rows.truncate(limit);

    let max_magnitude = rows
        .iter()
        .map(PositioningFlowRow::magnitude)
        .fold(0.0_f64, f64::max);

    PositioningFlowData {
        rows,
        max_magnitude,
        long_flow,
        short_flow,
        usd_available: live_mark.is_some(),
    }
}

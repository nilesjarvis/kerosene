use super::change::positioning_previous_change_size;
use super::live::positioning_live_change_usd;
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
    /// Signed change in size over the timeframe (+ = more long).
    pub(in crate::market_views::positioning_info) delta_size: f64,
    /// Signed current size (+ = long).
    pub(in crate::market_views::positioning_info) current_size: f64,
    /// Signed previous size, when derivable.
    pub(in crate::market_views::positioning_info) previous_size: Option<f64>,
    /// Signed change valued at the live mark, in USD (+ = more long).
    pub(in crate::market_views::positioning_info) delta_usd: Option<f64>,
    /// Signed current exposure valued at the live mark, in USD.
    pub(in crate::market_views::positioning_info) current_usd: Option<f64>,
    pub(in crate::market_views::positioning_info) kind: PositioningFlowKind,
}

impl PositioningFlowRow {
    /// Magnitude used for bar scaling and sorting. Prefers USD; falls back to
    /// raw size when no live mark is available so the view still ranks moves.
    pub(in crate::market_views::positioning_info) fn magnitude(&self) -> f64 {
        match self.delta_usd {
            Some(usd) if usd.is_finite() => usd.abs(),
            _ => self.delta_size.abs(),
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::market_views::positioning_info) struct PositioningFlowData {
    pub(in crate::market_views::positioning_info) rows: Vec<PositioningFlowRow>,
    /// Largest row magnitude, used to scale bars. Always finite and > 0 when
    /// there is at least one row with a measurable move.
    pub(in crate::market_views::positioning_info) max_magnitude: f64,
    /// Aggregate long-ward delta in USD (or size units when no mark).
    pub(in crate::market_views::positioning_info) long_flow: f64,
    /// Aggregate short-ward delta in USD (or size units when no mark).
    pub(in crate::market_views::positioning_info) short_flow: f64,
    /// True when magnitudes are expressed in USD rather than raw size.
    pub(in crate::market_views::positioning_info) usd_scaled: bool,
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
    let usd_scaled = live_mark.is_some();
    let mut rows: Vec<PositioningFlowRow> = Vec::with_capacity(deltas.len());
    let mut long_flow = 0.0;
    let mut short_flow = 0.0;

    for entry in deltas {
        if !entry.current.is_finite() || !entry.delta.is_finite() {
            continue;
        }
        let previous_size = positioning_previous_change_size(entry);
        let delta_usd = positioning_live_change_usd(entry.delta, live_mark);
        let current_usd = positioning_live_change_usd(entry.current, live_mark);
        let kind = classify_flow(previous_size, entry.current, entry.delta);

        let flow_value = match delta_usd {
            Some(usd) if usd.is_finite() => usd,
            _ => entry.delta,
        };
        if flow_value > 0.0 {
            long_flow += flow_value;
        } else if flow_value < 0.0 {
            short_flow += flow_value.abs();
        }

        rows.push(PositioningFlowRow {
            address: entry.address.clone(),
            delta_size: entry.delta,
            current_size: entry.current,
            previous_size,
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
        usd_scaled,
    }
}

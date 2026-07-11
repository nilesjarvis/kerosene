use super::super::sort::PositionRowData;
use crate::helpers::finite_value;

use std::fmt;

// ---------------------------------------------------------------------------
// Summary Totals
// ---------------------------------------------------------------------------

#[derive(Default, Clone, Copy, PartialEq)]
pub(super) struct PositionSummaryTotals {
    pub(super) funding_gross: OptionalTotal,
    pub(super) long_notional: f64,
    pub(super) short_notional: f64,
    pub(super) net_funding: OptionalTotal,
    pub(super) upnl: CompleteTotal,
    pub(super) total_pnl: CompleteTotal,
}

impl fmt::Debug for PositionSummaryTotals {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PositionSummaryTotals")
            .field("funding_gross", &self.funding_gross)
            .field("long_notional", &"<redacted>")
            .field("short_notional", &"<redacted>")
            .field("net_funding", &self.net_funding)
            .field("upnl", &self.upnl)
            .field("total_pnl", &self.total_pnl)
            .finish()
    }
}

impl PositionSummaryTotals {
    pub(super) fn from_rows(rows: impl IntoIterator<Item = PositionRowData>) -> Self {
        rows.into_iter().fold(Self::default(), |mut totals, row| {
            totals.add_row(row);
            totals
        })
    }

    fn add_row(&mut self, row: PositionRowData) {
        self.add_position(
            row.is_long,
            row.position_value,
            row.funding_since_open,
            row.upnl,
            row.total_pnl,
        );
    }

    pub(super) fn add_position(
        &mut self,
        is_long: Option<bool>,
        position_value: Option<f64>,
        funding_since_open: Option<f64>,
        upnl: Option<f64>,
        total_pnl: Option<f64>,
    ) {
        if let (Some(is_long), Some(position_value)) = (is_long, position_value) {
            if is_long {
                self.long_notional += position_value.abs();
            } else {
                self.short_notional += position_value.abs();
            }
        }

        self.funding_gross.add(funding_since_open.map(f64::abs));
        self.net_funding.add(funding_since_open);
        self.upnl.add(upnl);
        self.total_pnl.add(total_pnl);
    }
}

/// Total over rows where a missing value means "not applicable" (e.g. spot
/// rows have no funding), so absent rows are skipped and the sum over the
/// contributing rows is still displayable.
#[derive(Default, Clone, Copy, PartialEq)]
pub(super) struct OptionalTotal {
    value: f64,
    count: usize,
}

impl fmt::Debug for OptionalTotal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OptionalTotal")
            .field("value", &"<redacted>")
            .field("count", &self.count)
            .finish()
    }
}

impl OptionalTotal {
    pub(super) fn add(&mut self, value: Option<f64>) {
        if let Some(value) = value.and_then(finite_value) {
            self.value += value;
            self.count += 1;
        }
    }

    pub(super) fn value(self) -> Option<f64> {
        (self.count > 0).then_some(self.value)
    }
}

/// Total over rows where a missing value means "unknown" (e.g. a spot
/// position whose fill-derived cost basis is momentarily unavailable while a
/// trade settles). One unknown row makes the whole total unknown: summing
/// the rest would display a figure that is wrong by the missing position's
/// PnL, and no PnL must be shown over a possibly-wrong one.
#[derive(Default, Clone, Copy, PartialEq)]
pub(super) struct CompleteTotal {
    value: f64,
    count: usize,
    missing: usize,
}

impl fmt::Debug for CompleteTotal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompleteTotal")
            .field("value", &"<redacted>")
            .field("count", &self.count)
            .field("missing", &self.missing)
            .finish()
    }
}

impl CompleteTotal {
    pub(super) fn add(&mut self, value: Option<f64>) {
        match value.and_then(finite_value) {
            Some(value) => {
                self.value += value;
                self.count += 1;
            }
            None => self.missing += 1,
        }
    }

    pub(super) fn value(self) -> Option<f64> {
        (self.count > 0 && self.missing == 0).then_some(self.value)
    }
}

pub(super) fn position_total_pnl_percent(
    total_pnl: Option<f64>,
    account_balance: Option<f64>,
) -> Option<f64> {
    match (total_pnl, account_balance) {
        (Some(total_pnl), Some(account_balance)) if account_balance.abs() > f64::EPSILON => {
            Some(total_pnl / account_balance * 100.0)
        }
        _ => None,
    }
}

use super::super::sort::PositionRowData;
use crate::helpers::finite_value;

// ---------------------------------------------------------------------------
// Summary Totals
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(super) struct PositionSummaryTotals {
    pub(super) funding_gross: OptionalTotal,
    pub(super) long_notional: f64,
    pub(super) short_notional: f64,
    pub(super) net_funding: OptionalTotal,
    pub(super) upnl: OptionalTotal,
    pub(super) total_pnl: OptionalTotal,
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

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(super) struct OptionalTotal {
    value: f64,
    count: usize,
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

pub(super) fn position_total_pnl_percent(
    total_pnl: OptionalTotal,
    account_balance: Option<f64>,
) -> Option<f64> {
    match (total_pnl.value(), account_balance) {
        (Some(total_pnl), Some(account_balance)) if account_balance.abs() > f64::EPSILON => {
            Some(total_pnl / account_balance * 100.0)
        }
        _ => None,
    }
}

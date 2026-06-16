use super::super::sort::PositionRowData;
use crate::account::position_upnl_from_mark_or_wire;
use crate::helpers::{finite_value, parse_finite_number};

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

pub(super) fn parse_summary_number(raw: &str) -> Option<f64> {
    parse_finite_number(raw)
}

pub(super) fn position_summary_position_upnl_value(
    szi_raw: &str,
    entry_raw: &str,
    wire_upnl_raw: &str,
    live_mid: Option<f64>,
) -> Option<f64> {
    position_upnl_from_mark_or_wire(
        parse_summary_number(szi_raw),
        parse_summary_number(entry_raw),
        parse_summary_number(wire_upnl_raw),
        live_mid,
    )
}

pub(super) fn position_summary_spot_balance_value(
    coin: &str,
    total_raw: &str,
    entry_ntl_raw: &str,
    live_mid: Option<f64>,
) -> Option<f64> {
    let total = parse_summary_number(total_raw)?;
    if total.abs() < 1e-12 {
        return Some(0.0);
    }
    if matches!(coin, "USDC" | "USDE" | "USDT0" | "USDH") {
        Some(total)
    } else if let Some(mid) = live_mid {
        Some(total * mid)
    } else {
        parse_summary_number(entry_ntl_raw)
    }
}

pub(super) fn sum_required(values: impl IntoIterator<Item = Option<f64>>) -> Option<f64> {
    let mut total = 0.0;
    for value in values {
        total += value?;
    }
    Some(total)
}

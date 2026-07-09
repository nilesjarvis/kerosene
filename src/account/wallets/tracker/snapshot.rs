use super::super::super::{AssetPosition, WalletTrackerSnapshot};
use crate::account::{position_notional_from_mark_or_wire, position_upnl_from_mark_or_wire};
use crate::helpers::{add_optional_f64, parse_finite_number};

pub(super) fn build_wallet_tracker_snapshot(
    equity: Option<f64>,
    withdrawable: Option<f64>,
    margin_used: Option<f64>,
    asset_positions: Vec<AssetPosition>,
) -> WalletTrackerSnapshot {
    let mut unrealized_pnl = Some(0.0);
    let mut long_exposure = Some(0.0);
    let mut short_exposure = Some(0.0);
    let mut open_trade_count = Some(0_usize);
    for asset_position in &asset_positions {
        let position = &asset_position.position;
        let Some(szi) = parse_tracker_number(&position.szi) else {
            open_trade_count = None;
            long_exposure = None;
            short_exposure = None;
            unrealized_pnl = None;
            continue;
        };
        if szi.abs() <= 1e-12 {
            continue;
        }
        if let Some(count) = open_trade_count.as_mut() {
            *count += 1;
        }
        let position_value = position_notional_from_mark_or_wire(
            Some(szi),
            parse_tracker_number(&position.position_value),
            None,
        );
        if szi > 0.0 {
            add_optional_f64(&mut long_exposure, position_value);
        } else {
            add_optional_f64(&mut short_exposure, position_value);
        }
        add_optional_f64(
            &mut unrealized_pnl,
            position_upnl_from_mark_or_wire(
                Some(szi),
                None,
                parse_tracker_number(&position.unrealized_pnl),
                None,
            ),
        );
    }

    let margin_used_pct = margin_used_pct(equity, margin_used);

    WalletTrackerSnapshot {
        equity,
        withdrawable,
        unrealized_pnl,
        margin_used_pct,
        open_trade_count,
        open_order_count: 0,
        long_exposure,
        short_exposure,
        valuation_warning: None,
    }
}

pub(super) fn parse_tracker_number(value: &str) -> Option<f64> {
    parse_finite_number(value)
}

fn margin_used_pct(equity: Option<f64>, margin_used: Option<f64>) -> Option<f64> {
    match (equity, margin_used) {
        (Some(equity), Some(margin_used)) if equity > 0.0 => {
            Some((margin_used / equity * 100.0).max(0.0))
        }
        (Some(equity), Some(margin_used))
            if equity.abs() <= f64::EPSILON && margin_used.abs() <= f64::EPSILON =>
        {
            Some(0.0)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;

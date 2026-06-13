use crate::account::AccountData;
use crate::account::{position_notional_from_mark_or_wire, position_upnl_from_mark_or_wire};
use crate::helpers::parse_finite_number;

// ---------------------------------------------------------------------------
// Connected Summary Calculations
// ---------------------------------------------------------------------------

pub(in crate::account_views::summary::connected) fn parse_summary_number(raw: &str) -> Option<f64> {
    parse_finite_number(raw)
}

pub(in crate::account_views::summary::connected) fn position_upnl_value(
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

pub(in crate::account_views::summary::connected) fn position_notional_value(
    szi_raw: &str,
    wire_value_raw: &str,
    live_mid: Option<f64>,
) -> Option<f64> {
    position_notional_from_mark_or_wire(
        parse_summary_number(szi_raw),
        parse_summary_number(wire_value_raw),
        live_mid,
    )
}

pub(in crate::account_views::summary::connected) fn effective_leverage(
    notional: Option<f64>,
    account_value: Option<f64>,
) -> Option<f64> {
    match (notional, account_value) {
        (Some(notional), Some(account_value))
            if notional.abs() <= f64::EPSILON && account_value.abs() <= f64::EPSILON =>
        {
            Some(0.0)
        }
        (Some(notional), Some(account_value)) if account_value > 0.0 => {
            Some((notional.abs() / account_value).max(0.0))
        }
        _ => None,
    }
}

pub(in crate::account_views::summary::connected) fn spot_balance_value(
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

pub(in crate::account_views::summary::connected) fn shared_account_total_value(
    data: &AccountData,
    spot_value: impl FnOnce() -> Option<f64>,
) -> Option<f64> {
    if data.is_portfolio_margin() {
        spot_value().or_else(|| data.account_value_usdc())
    } else {
        match (data.account_value_usdc(), spot_value()) {
            (Some(account_value), Some(spot_value)) => Some(account_value.max(spot_value)),
            (Some(account_value), None) => Some(account_value),
            (None, Some(spot_value)) => Some(spot_value),
            (None, None) => None,
        }
    }
}

pub(in crate::account_views::summary::connected) fn shared_account_token_total_value(
    data: &AccountData,
    token: u32,
    resolve_mid: impl FnOnce(&str) -> Option<f64>,
) -> Option<f64> {
    let balance = data
        .spot
        .balances
        .iter()
        .find(|balance| balance.token == Some(token))
        .or_else(|| {
            if token == 0 {
                data.spot
                    .balances
                    .iter()
                    .find(|balance| balance.coin == "USDC")
            } else {
                None
            }
        })?;
    spot_balance_value(
        &balance.coin,
        &balance.total,
        &balance.entry_ntl,
        resolve_mid(&balance.coin),
    )
}

pub(in crate::account_views::summary::connected) fn sum_optional(
    values: impl IntoIterator<Item = Option<f64>>,
) -> Option<f64> {
    let mut total = 0.0;
    for value in values {
        total += value?;
    }
    Some(total)
}

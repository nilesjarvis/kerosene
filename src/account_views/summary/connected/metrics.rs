use crate::account::AccountData;
use crate::app_state::TradingTerminal;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Connected Summary Metrics
// ---------------------------------------------------------------------------

pub(in crate::account_views::summary::connected) struct ConnectedSummaryValues {
    pub(in crate::account_views::summary::connected) total_value: String,
    pub(in crate::account_views::summary::connected) available: Option<f64>,
    pub(in crate::account_views::summary::connected) available_value: String,
    pub(in crate::account_views::summary::connected) live_notional: String,
    pub(in crate::account_views::summary::connected) effective_leverage_value: String,
    pub(in crate::account_views::summary::connected) margin_used: Option<f64>,
    pub(in crate::account_views::summary::connected) margin_used_value: String,
    pub(in crate::account_views::summary::connected) portfolio_margin_ratio: Option<f64>,
    pub(in crate::account_views::summary::connected) portfolio_margin_ratio_value: String,
}

impl TradingTerminal {
    pub(super) fn connected_summary_values(&self, data: &AccountData) -> ConnectedSummaryValues {
        let clearinghouse = self.visible_clearinghouse_state(data);
        let include_spot = self.account_view_includes_spot_balances(data);
        let live_upnl = sum_optional(
            clearinghouse
                .asset_positions
                .iter()
                .filter(|ap| !self.symbol_key_is_hidden(&ap.position.coin))
                .map(|ap| {
                    position_upnl_value(
                        &ap.position.szi,
                        &ap.position.entry_px,
                        &ap.position.unrealized_pnl,
                        self.resolve_mid_for_symbol(&ap.position.coin),
                    )
                }),
        );

        let live_ntl = sum_optional(
            clearinghouse
                .asset_positions
                .iter()
                .filter(|ap| !self.symbol_key_is_hidden(&ap.position.coin))
                .map(|ap| {
                    position_notional_value(
                        &ap.position.szi,
                        &ap.position.position_value,
                        self.resolve_mid_for_symbol(&ap.position.coin),
                    )
                }),
        );

        let spot_value = if include_spot {
            sum_optional(
                data.spot
                    .balances
                    .iter()
                    .filter(|b| !self.account_spot_balance_is_hidden(data, &b.coin))
                    .map(|b| {
                        spot_balance_value(
                            &b.coin,
                            &b.total,
                            &b.entry_ntl,
                            self.resolve_mid_for_symbol(&b.coin),
                        )
                    }),
            )
        } else {
            Some(0.0)
        };

        let stale_upnl = sum_optional(
            clearinghouse
                .asset_positions
                .iter()
                .filter(|ap| !self.symbol_key_is_hidden(&ap.position.coin))
                .map(|ap| parse_summary_number(&ap.position.unrealized_pnl)),
        );
        let balances_can_be_sized = !matches!(
            data.account_abstraction,
            crate::account::AccountAbstractionMode::Unknown(_)
        );
        let total_value = if !balances_can_be_sized {
            None
        } else if data.uses_shared_account_balance() && !include_spot {
            self.visible_collateral_token().and_then(|token| {
                shared_account_token_total_value(data, token, |coin| {
                    self.resolve_mid_for_symbol(coin)
                })
            })
        } else if data.uses_shared_account_balance() {
            shared_account_total_value(data, || {
                sum_optional(data.spot.balances.iter().map(|balance| {
                    spot_balance_value(
                        &balance.coin,
                        &balance.total,
                        &balance.entry_ntl,
                        self.resolve_mid_for_symbol(&balance.coin),
                    )
                }))
            })
        } else {
            let perp_equity = parse_summary_number(&clearinghouse.margin_summary.account_value);
            match (perp_equity, spot_value, live_upnl, stale_upnl) {
                (Some(perp_equity), Some(spot_value), Some(live_upnl), Some(stale_upnl)) => {
                    Some(perp_equity + spot_value + (live_upnl - stale_upnl))
                }
                _ => None,
            }
        };

        let available = if !balances_can_be_sized {
            None
        } else if data.is_portfolio_margin() {
            data.available_margin_usdc()
        } else if data.uses_shared_account_balance() {
            self.visible_collateral_token()
                .and_then(|token| data.available_margin_for_token(token))
        } else {
            match (
                parse_summary_number(&clearinghouse.withdrawable),
                live_upnl,
                stale_upnl,
            ) {
                (Some(withdrawable), Some(live_upnl), Some(stale_upnl)) => {
                    Some(withdrawable + (live_upnl - stale_upnl))
                }
                _ => None,
            }
        };
        let margin_used = parse_summary_number(&clearinghouse.margin_summary.total_margin_used);
        let effective_leverage = effective_leverage(live_ntl, total_value);
        let portfolio_margin_ratio = data
            .is_portfolio_margin()
            .then(|| {
                data.spot
                    .portfolio_margin_ratio
                    .as_deref()
                    .and_then(parse_summary_number)
            })
            .flatten();

        ConnectedSummaryValues {
            total_value: summary_number_string(total_value),
            available,
            available_value: summary_number_string(available),
            live_notional: summary_number_string(live_ntl),
            effective_leverage_value: leverage_string(effective_leverage),
            margin_used,
            margin_used_value: summary_number_string(margin_used),
            portfolio_margin_ratio,
            portfolio_margin_ratio_value: summary_percent_string(portfolio_margin_ratio),
        }
    }
}

fn parse_summary_number(raw: &str) -> Option<f64> {
    raw.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn position_upnl_value(
    szi_raw: &str,
    entry_raw: &str,
    wire_upnl_raw: &str,
    live_mid: Option<f64>,
) -> Option<f64> {
    match (
        live_mid,
        parse_summary_number(szi_raw),
        parse_summary_number(entry_raw),
    ) {
        (Some(mid), Some(szi), Some(entry)) => Some(szi * (mid - entry)),
        _ => parse_summary_number(wire_upnl_raw),
    }
}

fn position_notional_value(
    szi_raw: &str,
    wire_value_raw: &str,
    live_mid: Option<f64>,
) -> Option<f64> {
    match (live_mid, parse_summary_number(szi_raw)) {
        (Some(mid), Some(szi)) => Some(szi.abs() * mid),
        _ => parse_summary_number(wire_value_raw).map(f64::abs),
    }
}

fn effective_leverage(notional: Option<f64>, account_value: Option<f64>) -> Option<f64> {
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

fn spot_balance_value(
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

fn shared_account_total_value(
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

fn shared_account_token_total_value(
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

fn sum_optional(values: impl IntoIterator<Item = Option<f64>>) -> Option<f64> {
    let mut total = 0.0;
    for value in values {
        total += value?;
    }
    Some(total)
}

fn summary_number_string(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "Invalid data".to_string())
}

fn summary_percent_string(value: Option<f64>) -> String {
    value
        .map(|value| format!("{:.2}%", value * 100.0))
        .unwrap_or_else(|| "Invalid data".to_string())
}

fn leverage_string(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}x"))
        .unwrap_or_else(|| "Invalid data".to_string())
}

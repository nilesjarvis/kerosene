use crate::account::AccountData;
use crate::app_state::TradingTerminal;

mod calculations;
mod formatting;

use calculations::*;
use formatting::*;

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

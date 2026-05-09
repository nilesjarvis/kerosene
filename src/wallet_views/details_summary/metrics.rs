use crate::account::WalletDetailsData;
use crate::app_state::TradingTerminal;
use crate::helpers::format_usd;
use crate::wallet_views::numbers::{parse_wallet_number, wallet_has_visible_nonzero};

use super::WalletDetailsSummaryMetrics;

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(super) fn wallet_details_summary_metrics(
        &self,
        data: &WalletDetailsData,
    ) -> WalletDetailsSummaryMetrics {
        let account_value = parse_wallet_number(&data.clearinghouse.margin_summary.account_value);
        let withdrawable = parse_wallet_number(&data.clearinghouse.withdrawable);
        let margin_used = parse_wallet_number(&data.clearinghouse.margin_summary.total_margin_used);
        let notional = parse_wallet_number(&data.clearinghouse.margin_summary.total_ntl_pos);
        let margin_pct = wallet_margin_pct(account_value, margin_used);

        let mut long_exposure = Some(0.0);
        let mut short_exposure = Some(0.0);
        let mut unrealized_pnl = Some(0.0);
        let mut active_position_count = 0usize;
        for detail in &data.positions {
            let pos = &detail.asset_position.position;
            let symbol = Self::wallet_detail_symbol(&detail.dex, &pos.coin);
            if self.is_ticker_muted(&symbol) || self.is_ticker_muted(&pos.coin) {
                continue;
            }
            if !wallet_has_visible_nonzero(&pos.szi) {
                continue;
            }
            active_position_count += 1;
            let szi = parse_wallet_number(&pos.szi);
            let entry_px = parse_wallet_number(&pos.entry_px);
            let mark_px = self
                .resolve_mid_for_symbol(&symbol)
                .or_else(|| self.resolve_mid_for_symbol(&pos.coin));
            let position_value = wallet_position_value(szi, &pos.position_value, mark_px);
            let row_upnl = wallet_position_upnl(szi, entry_px, &pos.unrealized_pnl, mark_px);
            add_optional(&mut unrealized_pnl, row_upnl);
            match szi {
                Some(szi) if szi > 0.0 => add_optional(&mut long_exposure, position_value),
                Some(_) => add_optional(&mut short_exposure, position_value),
                None => {
                    long_exposure = None;
                    short_exposure = None;
                }
            }
        }

        let open_order_count = data
            .open_orders
            .iter()
            .filter(|detail| {
                let symbol = Self::wallet_detail_symbol(&detail.dex, &detail.order.coin);
                !self.is_ticker_muted(&symbol) && !self.is_ticker_muted(&detail.order.coin)
            })
            .count();

        let non_zero_spot_count = data
            .spot
            .balances
            .iter()
            .filter(|balance| {
                wallet_has_visible_nonzero(&balance.total) && !self.is_ticker_muted(&balance.coin)
            })
            .count();
        let pm_ratio = data
            .spot
            .portfolio_margin_ratio
            .as_deref()
            .and_then(parse_wallet_number);
        let pm_available_raw = data
            .spot
            .token_to_available_after_maintenance
            .as_ref()
            .and_then(|tokens| {
                tokens
                    .iter()
                    .find(|(token, _)| *token == 0)
                    .map(|(_, amount)| amount.as_str())
            });
        let pm_available = match pm_available_raw {
            Some(amount) => parse_wallet_number(amount)
                .map(|amount| format_usd(&amount.to_string()))
                .unwrap_or_else(|| "Invalid data".to_string()),
            None => "-".to_string(),
        };

        WalletDetailsSummaryMetrics {
            account_value,
            withdrawable,
            margin_pct,
            notional,
            long_exposure,
            short_exposure,
            unrealized_pnl,
            active_position_count,
            open_order_count,
            non_zero_spot_count,
            pm_ratio,
            pm_available,
            portfolio_margin_enabled: data.spot.portfolio_margin_enabled,
        }
    }
}

fn wallet_margin_pct(account_value: Option<f64>, margin_used: Option<f64>) -> Option<f64> {
    match (account_value, margin_used) {
        (Some(account_value), Some(margin_used)) if account_value.abs() > f64::EPSILON => {
            Some(margin_used / account_value * 100.0)
        }
        (Some(account_value), Some(margin_used))
            if account_value.abs() <= f64::EPSILON && margin_used.abs() <= f64::EPSILON =>
        {
            Some(0.0)
        }
        _ => None,
    }
}

fn wallet_position_value(
    szi: Option<f64>,
    wire_position_value: &str,
    mark_px: Option<f64>,
) -> Option<f64> {
    match (szi, mark_px) {
        (Some(szi), Some(mark_px)) => Some(szi.abs() * mark_px),
        _ => parse_wallet_number(wire_position_value).map(f64::abs),
    }
}

fn wallet_position_upnl(
    szi: Option<f64>,
    entry_px: Option<f64>,
    wire_upnl: &str,
    mark_px: Option<f64>,
) -> Option<f64> {
    match (szi, entry_px, mark_px) {
        (Some(szi), Some(entry_px), Some(mark_px)) => Some(szi * (mark_px - entry_px)),
        _ => parse_wallet_number(wire_upnl),
    }
}

fn add_optional(total: &mut Option<f64>, value: Option<f64>) {
    *total = total.and_then(|total| value.map(|value| total + value));
}

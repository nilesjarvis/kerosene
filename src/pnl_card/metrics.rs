use crate::account;
use crate::app_state::TradingTerminal;

mod numbers;

use numbers::PositionCardNumbers;
#[cfg(test)]
pub(super) use numbers::mark_from_wire_upnl;
pub(super) use numbers::{pct_from_basis, position_asset_move_pct};

// ---------------------------------------------------------------------------
// PnL Card Metrics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(super) struct PnlCardMetrics {
    pub(super) ticker: String,
    pub(super) leverage_display: String,
    pub(super) entry_display: String,
    pub(super) exit_display: String,
    pub(super) context: String,
    pub(super) private_context: Option<String>,
    pub(super) upnl: f64,
    pub(super) asset_move_pct: Option<f64>,
    pub(super) leveraged_pct: Option<f64>,
}

impl TradingTerminal {
    pub(super) fn position_pnl_card_metrics(&self, coin: &str) -> Option<PnlCardMetrics> {
        let ap = self.pnl_card_position_for_coin(coin)?;
        let pos = &ap.position;
        let numbers = PositionCardNumbers::from_position(self, pos)?;
        let side = if numbers.szi >= 0.0 { "Long" } else { "Short" };
        let leverage = pos.leverage.value.max(1);

        Some(PnlCardMetrics {
            ticker: self.display_name_for_symbol(&pos.coin),
            leverage_display: format!("{leverage}x"),
            entry_display: self.format_display_price(numbers.entry_px),
            exit_display: self.format_display_price(numbers.mark_px),
            context: format!(
                "{side} {}",
                self.display_size_for_symbol(&pos.coin, numbers.szi.abs())
            ),
            private_context: Some(format!("{side} position")),
            upnl: numbers.upnl,
            asset_move_pct: position_asset_move_pct(numbers.szi, numbers.entry_px, numbers.mark_px),
            leveraged_pct: position_asset_move_pct(numbers.szi, numbers.entry_px, numbers.mark_px)
                .map(|pct| pct * f64::from(leverage)),
        })
    }

    pub(super) fn summary_pnl_card_metrics(&self) -> Option<PnlCardMetrics> {
        let mut count = 0usize;
        let mut upnl = 0.0;
        let mut entry_notional = 0.0;
        let mut margin_basis = 0.0;
        let mut weighted_leverage = 0.0;

        for ap in self.visible_pnl_card_positions() {
            let pos = &ap.position;
            let Some(numbers) = PositionCardNumbers::from_position(self, pos) else {
                continue;
            };
            let leverage = f64::from(pos.leverage.value.max(1));
            let position_entry_notional = numbers.szi.abs() * numbers.entry_px.abs();
            let margin = if numbers.margin_used > 0.0 {
                numbers.margin_used
            } else {
                position_entry_notional / leverage
            };

            count += 1;
            upnl += numbers.upnl;
            entry_notional += position_entry_notional;
            margin_basis += margin;
            weighted_leverage += leverage * position_entry_notional;
        }

        if count == 0 {
            return None;
        }

        let avg_leverage = if entry_notional > f64::EPSILON {
            weighted_leverage / entry_notional
        } else {
            0.0
        };

        Some(PnlCardMetrics {
            ticker: "PORTFOLIO".to_string(),
            leverage_display: if avg_leverage > 0.0 {
                format!("{avg_leverage:.1}x avg")
            } else {
                "Mixed".to_string()
            },
            entry_display: "Mixed".to_string(),
            exit_display: "Live marks".to_string(),
            context: format!("{count} open position{}", if count == 1 { "" } else { "s" }),
            private_context: None,
            upnl,
            asset_move_pct: pct_from_basis(upnl, entry_notional),
            leveraged_pct: pct_from_basis(upnl, margin_basis),
        })
    }

    pub(super) fn visible_pnl_card_positions(
        &self,
    ) -> impl Iterator<Item = &account::AssetPosition> {
        self.account_data
            .as_ref()
            .into_iter()
            .flat_map(|data| data.clearinghouse.asset_positions.iter())
            .filter(|ap| {
                !self.symbol_key_is_hidden(&ap.position.coin)
                    && (self.show_hidden_positions || !self.position_is_hidden(&ap.position.coin))
            })
    }

    pub(super) fn pnl_card_position_for_coin(&self, coin: &str) -> Option<account::AssetPosition> {
        self.account_positions_with_outcomes()
            .into_iter()
            .find(|ap| ap.position.coin == coin)
    }
}

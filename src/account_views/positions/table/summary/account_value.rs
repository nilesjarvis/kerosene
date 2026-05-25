use super::totals::{
    parse_summary_number, position_summary_position_upnl_value,
    position_summary_spot_balance_value, sum_required,
};
use crate::account;
use crate::app_state::TradingTerminal;

// ---------------------------------------------------------------------------
// Account Value
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn position_summary_account_value(
        &self,
        data: &account::AccountData,
    ) -> Option<f64> {
        let clearinghouse = self.visible_clearinghouse_state(data);
        let include_spot = self.account_view_includes_spot_balances(data);
        let live_upnl = sum_required(
            clearinghouse
                .asset_positions
                .iter()
                .filter(|ap| !self.symbol_key_is_hidden(&ap.position.coin))
                .map(|ap| {
                    position_summary_position_upnl_value(
                        &ap.position.szi,
                        &ap.position.entry_px,
                        &ap.position.unrealized_pnl,
                        self.resolve_mid_for_symbol(&ap.position.coin),
                    )
                }),
        );
        let stale_upnl = sum_required(
            clearinghouse
                .asset_positions
                .iter()
                .filter(|ap| !self.symbol_key_is_hidden(&ap.position.coin))
                .map(|ap| parse_summary_number(&ap.position.unrealized_pnl)),
        );
        let spot_value = if include_spot {
            sum_required(
                data.spot
                    .balances
                    .iter()
                    .filter(|balance| !self.account_spot_balance_is_hidden(data, &balance.coin))
                    .map(|balance| {
                        position_summary_spot_balance_value(
                            &balance.coin,
                            &balance.total,
                            &balance.entry_ntl,
                            self.resolve_mid_for_symbol(&balance.coin),
                        )
                    }),
            )
        } else {
            Some(0.0)
        };
        let perp_equity = if include_spot && data.is_portfolio_margin() {
            Some(0.0)
        } else {
            parse_summary_number(&clearinghouse.margin_summary.account_value)
        };

        match (perp_equity, spot_value, live_upnl, stale_upnl) {
            (Some(perp_equity), Some(spot_value), Some(live_upnl), Some(stale_upnl)) => {
                Some(perp_equity + spot_value + (live_upnl - stale_upnl))
            }
            _ => None,
        }
    }
}

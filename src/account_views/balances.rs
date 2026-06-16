mod row;
mod table;

use self::row::{balance_has_visible_total, balance_row};
use self::table::{balances_rows_table, empty_balances_table};

use crate::account::SpotBalance;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Element;

// ---------------------------------------------------------------------------
// Balances Table
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_balances(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let snapshot = self
            .connected_order_account_snapshot()
            .map(|(_, data)| data);
        let balances: Vec<_> = snapshot
            .map(|d| {
                d.spot
                    .balances
                    .iter()
                    .filter(|balance| !self.account_spot_balance_is_hidden(d, &balance.coin))
                    .collect()
            })
            .unwrap_or_default();

        if balances.is_empty() {
            let msg = if self.connected_address.is_some() {
                "No balances"
            } else {
                "Connect wallet to view balances"
            };
            return empty_balances_table(msg, &theme);
        }

        // Filter to non-zero balances only.
        let non_zero: Vec<&SpotBalance> = balances
            .iter()
            .copied()
            .filter(|balance| balance_has_visible_total(balance))
            .collect();

        if non_zero.is_empty() {
            return empty_balances_table("No balances", &theme);
        }

        let rows = non_zero
            .iter()
            .fold(iced::widget::Column::new().spacing(2), |col, bal| {
                let display_coin = self.display_coin_for_spot_balance(&bal.coin);
                let outcome_sell_coin = self.outcome_trade_coin_for_balance_coin(&bal.coin);
                col.push(balance_row(
                    bal,
                    display_coin,
                    outcome_sell_coin,
                    &self.display_denomination_context(),
                    &theme,
                ))
            });

        balances_rows_table(rows)
    }
}

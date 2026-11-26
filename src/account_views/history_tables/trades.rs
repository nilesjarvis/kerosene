mod aggregation;
#[path = "trades/row.rs"]
mod trade_row;

use self::aggregation::trade_history_rows;
use crate::account::AccountDataSection;
use crate::account_views::table_helpers::{account_table_scroll, empty_account_table};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Column, checkbox, column, container, rule, text};
use iced::{Element, Fill};

// ---------------------------------------------------------------------------
// Trade History Table
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_trade_history(&self) -> Element<'_, Message> {
        let toggle = checkbox(self.aggregate_trade_history)
            .label("Aggregate fills")
            .size(13)
            .text_size(11)
            .spacing(5)
            .on_toggle(Message::SetTradeHistoryAggregation);
        column![
            container(toggle).align_right(Fill),
            self.view_trade_history_table()
        ]
        .spacing(4)
        .into()
    }

    fn view_trade_history_table(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let header = trade_row::view_trade_history_header(&theme, self.aggregate_trade_history);

        let snapshot = self
            .connected_order_account_snapshot()
            .map(|(_, data)| data);
        let fills = snapshot
            .into_iter()
            .flat_map(|data| &data.fills)
            .filter(|fill| !self.symbol_key_is_hidden(&fill.coin));
        let trades = trade_history_rows(fills, self.aggregate_trade_history);
        let warning =
            snapshot.and_then(|data| data.completeness.section_warning(AccountDataSection::Fills));

        if trades.is_empty() {
            let msg = if let Some(warning) = warning {
                warning
            } else if self.connected_address.is_some() {
                "No trade history".to_string()
            } else {
                "Connect wallet to view trades".to_string()
            };
            return empty_account_table(header, msg, &theme);
        }

        let rows = trades
            .into_iter()
            .fold(Column::new().spacing(2), |col, trade| {
                col.push(self.view_trade_history_row(trade, &theme))
            });

        let mut content = column![header].spacing(4);
        if let Some(warning) = warning {
            content = content.push(text(warning).size(11).color(theme.palette().warning));
        }
        let content = content.push(rule::horizontal(1)).push(rows);
        account_table_scroll(content)
    }
}

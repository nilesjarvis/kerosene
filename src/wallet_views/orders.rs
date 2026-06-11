mod row;
#[cfg(test)]
mod tests;

use self::row::{wallet_order_row, wallet_orders_header};

use crate::account::WalletOpenOrderDetail;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, container, rule, text};
use iced::{Color, Element, Fill, Theme};
use std::cmp::Reverse;

// ---------------------------------------------------------------------------
// Wallet Detail Orders
// ---------------------------------------------------------------------------

impl TradingTerminal {
    /// Outcome and spot order coins resolve to their human labels; perp and
    /// HIP-3 coins keep the dex-qualified key.
    fn wallet_order_symbol_label(&self, dex: &str, coin: &str) -> String {
        if self.is_outcome_coin(coin) || coin.starts_with('@') {
            self.display_name_for_symbol(coin)
        } else {
            Self::wallet_detail_symbol(dex, coin)
        }
    }

    pub(super) fn view_wallet_orders_table<'a>(
        &'a self,
        orders: &'a [WalletOpenOrderDetail],
        now_ms: u64,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let mut order_rows: Vec<&WalletOpenOrderDetail> = orders
            .iter()
            .filter(|detail| {
                self.visible_wallet_detail_symbol(&detail.dex, &detail.order.coin)
                    .is_some()
            })
            .collect();
        order_rows.sort_by_key(|detail| Reverse(detail.order.timestamp));
        let mut orders_table = Column::new()
            .spacing(4)
            .push(text("Open Orders").size(13).color(theme.palette().text))
            .push(wallet_orders_header())
            .push(rule::horizontal(1));
        if order_rows.is_empty() {
            orders_table = orders_table.push(
                text("No open orders")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            );
        } else {
            let denomination = self.display_denomination_context();
            for detail in order_rows.into_iter().take(80) {
                let symbol_label = self.wallet_order_symbol_label(&detail.dex, &detail.order.coin);
                let is_outcome = self.is_outcome_coin(&detail.order.coin);
                orders_table = orders_table.push(wallet_order_row(
                    detail,
                    symbol_label,
                    is_outcome,
                    now_ms,
                    &denomination,
                    &theme,
                ));
            }
        }

        container(orders_table)
            .padding([8, 8])
            .width(Fill)
            .style(|theme: &Theme| container_style::Style {
                background: Some(
                    Color {
                        a: 0.22,
                        ..theme.extended_palette().background.weak.color
                    }
                    .into(),
                ),
                border: iced::Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: theme.extended_palette().background.strong.color,
                },
                ..Default::default()
            })
            .into()
    }
}

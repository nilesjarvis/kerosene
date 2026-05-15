mod row;

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
    pub(super) fn view_wallet_orders_table<'a>(
        &'a self,
        orders: &'a [WalletOpenOrderDetail],
        now_ms: u64,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let mut order_rows: Vec<&WalletOpenOrderDetail> = orders
            .iter()
            .filter(|detail| {
                let symbol = Self::wallet_detail_symbol(&detail.dex, &detail.order.coin);
                !self.symbol_key_is_hidden(&symbol)
                    && !self.symbol_key_is_hidden(&detail.order.coin)
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
            for detail in order_rows.into_iter().take(80) {
                orders_table = orders_table.push(wallet_order_row(detail, now_ms, &theme));
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

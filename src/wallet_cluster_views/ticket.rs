use crate::app_state::TradingTerminal;
use crate::helpers::text_input_style;
use crate::message::Message;
use crate::signing::OrderKind;
use crate::wallet_cluster_state::{cluster_button_label, cluster_order_kind_options};
use iced::widget::{Space, button, checkbox, column, row, text, text_input};
use iced::{Alignment, Element, Fill, Length, Theme};

impl TradingTerminal {
    pub(super) fn view_wallet_cluster_ticket(&self, theme: &Theme) -> Element<'_, Message> {
        let order_kind = self.wallet_clusters.order_kind;
        let kind_row = cluster_order_kind_options().into_iter().fold(
            row![text("Order").size(13).color(theme.palette().text)]
                .spacing(6)
                .align_y(Alignment::Center),
            |row, kind| {
                let label = if kind == order_kind {
                    format!("{} *", cluster_button_label(kind))
                } else {
                    cluster_button_label(kind).to_string()
                };
                row.push(
                    button(text(label).size(11))
                        .padding([4, 8])
                        .on_press(Message::WalletClusterSetOrderKind(kind)),
                )
            },
        );
        let mut price_row = row![
            text(self.display_name_for_symbol(&self.active_symbol))
                .size(12)
                .width(Length::Fill),
            button(text("Mid").size(11))
                .padding([4, 8])
                .on_press(Message::WalletClusterSetMidPrice)
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        if !matches!(order_kind, OrderKind::Market) {
            price_row = price_row.push(
                text_input("Price", &self.wallet_clusters.order_price)
                    .style(text_input_style)
                    .on_input(|value| Message::WalletClusterOrderPriceChanged(value.into()))
                    .size(12)
                    .padding(6)
                    .width(Length::Fixed(130.0)),
            );
        }

        column![
            kind_row,
            price_row,
            row![
                text_input(
                    if self.wallet_clusters.order_quantity_is_usd {
                        "USDC size"
                    } else {
                        "Coin size"
                    },
                    &self.wallet_clusters.order_quantity,
                )
                .style(text_input_style)
                .on_input(|value| Message::WalletClusterOrderQuantityChanged(value.into()))
                .size(12)
                .padding(6)
                .width(Length::Fixed(150.0)),
                button(
                    text(if self.wallet_clusters.order_quantity_is_usd {
                        "USDC"
                    } else {
                        "Coin"
                    })
                    .size(11)
                )
                .padding([4, 8])
                .on_press(Message::WalletClusterToggleOrderDenomination),
                checkbox(self.wallet_clusters.reduce_only)
                    .label("Reduce only")
                    .on_toggle(|_| Message::WalletClusterToggleReduceOnly)
                    .size(12)
                    .spacing(6)
                    .text_size(12),
                Space::new().width(Fill),
                button(text("Buy Cluster").size(12))
                    .padding([6, 12])
                    .on_press(Message::WalletClusterSubmitOrder { is_buy: true }),
                button(text("Sell Cluster").size(12))
                    .padding([6, 12])
                    .on_press(Message::WalletClusterSubmitOrder { is_buy: false }),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(8)
        .into()
    }
}

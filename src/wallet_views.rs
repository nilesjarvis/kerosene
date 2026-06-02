mod address_actions;
mod details_header;
mod details_summary;
mod numbers;
mod orders;
mod position_metrics;
mod positions;
mod spot;
mod style;
mod tracker;
mod warnings;

pub(crate) use address_actions::{WalletAddressActionCell, wallet_address_action_cell};

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, container, rule, scrollable, text};
use iced::{Element, Fill, Theme, window};

#[cfg(test)]
mod tests;

pub(in crate::wallet_views) fn wallet_dex_label(dex: &str) -> String {
    if dex.is_empty() {
        "main".to_string()
    } else {
        dex.to_string()
    }
}

impl TradingTerminal {
    fn visible_wallet_detail_symbol(&self, dex: &str, coin: &str) -> Option<String> {
        let symbol = Self::wallet_detail_symbol(dex, coin);
        if self.symbol_key_is_hidden(&symbol) || self.symbol_key_is_hidden(coin) {
            None
        } else {
            Some(symbol)
        }
    }

    pub(crate) fn view_wallet_details(&self, window_id: window::Id) -> Element<'_, Message> {
        let theme = self.theme();
        let Some(state) = self.wallet_detail_windows.get(&window_id) else {
            return self.view_main();
        };

        let now_ms = Self::now_ms();
        let mut content = Column::new()
            .spacing(10)
            .push(self.view_wallet_details_header(window_id, state, now_ms, &theme))
            .push(rule::horizontal(1));

        if let Some(error) = state.error.as_ref() {
            content = content.push(
                container(text(error.clone()).size(12).color(theme.palette().danger))
                    .padding([6, 8])
                    .width(Fill),
            );
        }

        let Some(data) = state.data.as_ref() else {
            let message = if state.loading {
                "Loading wallet positioning..."
            } else {
                "No wallet detail snapshot loaded."
            };
            content = content.push(
                container(text(message).size(12).color(theme.palette().text))
                    .width(Fill)
                    .height(Fill)
                    .center_x(Fill)
                    .center_y(Fill),
            );
            return container(content)
                .padding(12)
                .width(Fill)
                .height(Fill)
                .style(|theme: &Theme| container_style::Style {
                    background: Some(theme.extended_palette().background.base.color.into()),
                    ..Default::default()
                })
                .into();
        };

        content = content.push(self.view_wallet_details_summary(data, &theme));

        content = content.push(self.view_wallet_positions_table(&data.positions));

        content = content.push(self.view_wallet_orders_table(&data.open_orders, now_ms));

        content = content.push(self.view_wallet_spot_table(&data.spot.balances));

        if let Some(warnings) = self.view_wallet_detail_warnings(&data.warnings, &theme) {
            content = content.push(warnings);
        }

        container(scrollable(content).height(Fill))
            .padding(12)
            .width(Fill)
            .height(Fill)
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.base.color.into()),
                ..Default::default()
            })
            .into()
    }
}

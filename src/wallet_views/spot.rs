mod row;

use self::row::{wallet_spot_header, wallet_spot_row};

use super::numbers::wallet_has_visible_nonzero;

use crate::account::SpotBalance;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, container, rule, text};
use iced::{Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Wallet Detail Spot Balances
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_wallet_spot_table<'a>(
        &'a self,
        balances: &'a [SpotBalance],
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let mut spot_rows: Vec<(&SpotBalance, String)> = balances
            .iter()
            .filter(|balance| {
                wallet_has_visible_nonzero(&balance.total)
                    && !self.symbol_key_is_hidden(&balance.coin)
            })
            .map(|balance| (balance, self.display_coin_for_spot_balance(&balance.coin)))
            .collect();
        spot_rows.sort_by(|a, b| a.1.cmp(&b.1));
        let mut spot_table = Column::new()
            .spacing(4)
            .push(text("Spot Balances").size(13).color(theme.palette().text))
            .push(wallet_spot_header())
            .push(rule::horizontal(1));
        if spot_rows.is_empty() {
            spot_table = spot_table.push(
                text("No spot balances")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            );
        } else {
            let denomination = self.display_denomination_context();
            for (balance, display_coin) in spot_rows.into_iter().take(80) {
                spot_table = spot_table.push(wallet_spot_row(
                    balance,
                    display_coin,
                    &denomination,
                    &theme,
                ));
            }
        }

        container(spot_table)
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

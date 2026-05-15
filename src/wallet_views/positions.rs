use crate::account::WalletPositionDetail;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, container, row, rule, text};
use iced::{Color, Element, Fill, Theme};

use super::numbers::wallet_has_visible_nonzero;

mod position_row;

// ---------------------------------------------------------------------------
// Wallet Detail Positions
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_wallet_positions_table<'a>(
        &'a self,
        positions: &'a [WalletPositionDetail],
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let mut position_rows: Vec<&WalletPositionDetail> = positions
            .iter()
            .filter(|detail| {
                let pos = &detail.asset_position.position;
                let symbol = Self::wallet_detail_symbol(&detail.dex, &pos.coin);
                wallet_has_visible_nonzero(&pos.szi)
                    && !self.symbol_key_is_hidden(&symbol)
                    && !self.symbol_key_is_hidden(&pos.coin)
            })
            .collect();
        position_rows.sort_by(|a, b| {
            let a_symbol = Self::wallet_detail_symbol(&a.dex, &a.asset_position.position.coin);
            let b_symbol = Self::wallet_detail_symbol(&b.dex, &b.asset_position.position.coin);
            a_symbol.cmp(&b_symbol)
        });

        let positions_header = row![
            text("Coin").size(10).width(95),
            text("Dex").size(10).width(60),
            text("Side").size(10).width(44),
            text("Size").size(10).width(84),
            text("Entry").size(10).width(78),
            text("Mark").size(10).width(78),
            text("Liq").size(10).width(78),
            text("Value").size(10).width(84),
            text("uPnL").size(10).width(84),
            text("Funding").size(10).width(84),
            text("Lev").size(10).width(44),
        ]
        .spacing(8);
        let mut positions_table = Column::new()
            .spacing(4)
            .push(text("Positions").size(13).color(theme.palette().text))
            .push(positions_header)
            .push(rule::horizontal(1));

        if position_rows.is_empty() {
            positions_table = positions_table.push(
                text("No open perp positions")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            );
        } else {
            for detail in position_rows {
                positions_table = positions_table.push(self.view_wallet_position_row(detail));
            }
        }

        container(positions_table)
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

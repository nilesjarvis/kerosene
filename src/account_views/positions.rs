mod header;
mod table;

use crate::account::{self, AccountDataSection};
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::{column, container, rule, scrollable, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_positions(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let can_close =
            self.connected_address.is_some() && !self.wallet_key_input.trim().is_empty();

        let positions: Vec<&account::AssetPosition> = self
            .account_data
            .as_ref()
            .map(|d| {
                d.clearinghouse
                    .asset_positions
                    .iter()
                    .filter(|ap| !self.is_ticker_muted(&ap.position.coin))
                    .collect()
            })
            .unwrap_or_default();
        let warning = self.account_data.as_ref().and_then(|data| {
            data.completeness
                .section_warning(AccountDataSection::Positions)
        });

        let header = self.view_positions_header(can_close, &positions, &theme);

        if positions.is_empty() {
            let msg = if let Some(warning) = warning {
                warning
            } else if self.connected_address.is_some() {
                "No open positions".to_string()
            } else {
                "Connect wallet to view positions".to_string()
            };
            let content = column![
                header,
                rule::horizontal(1),
                container(
                    text(msg)
                        .size(12)
                        .color(theme.extended_palette().background.weak.text)
                )
                .padding([8, 0]),
            ]
            .spacing(4);
            return positions_scrollable(content);
        }

        let rows = self.view_position_rows(&positions, can_close, &theme);
        let mut content = column![header].spacing(4);
        if let Some(warning) = warning {
            content = content.push(text(warning).size(11).color(theme.palette().warning));
        }
        let content = content.push(rule::horizontal(1)).push(rows);
        positions_scrollable(content)
    }
}

fn positions_scrollable<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    scrollable(content)
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new()
                .width(4)
                .margin(0)
                .scroller_width(4),
        ))
        .width(Fill)
        .height(Fill)
        .into()
}

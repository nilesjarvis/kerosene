#[path = "trades/row.rs"]
mod trade_row;

use crate::account::AccountDataSection;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Column, column, container, rule, scrollable, text};
use iced::{Element, Fill};

// ---------------------------------------------------------------------------
// Trade History Table
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_trade_history(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let header = trade_row::view_trade_history_header(&theme);

        let fills: Vec<_> = self
            .account_data
            .as_ref()
            .map(|d| {
                d.fills
                    .iter()
                    .filter(|fill| !self.is_ticker_muted(&fill.coin))
                    .collect()
            })
            .unwrap_or_default();
        let warning = self
            .account_data
            .as_ref()
            .and_then(|data| data.completeness.section_warning(AccountDataSection::Fills));

        if fills.is_empty() {
            let msg = if let Some(warning) = warning {
                warning
            } else if self.connected_address.is_some() {
                "No trade history".to_string()
            } else {
                "Connect wallet to view trades".to_string()
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
            return scrollable(content)
                .direction(iced::widget::scrollable::Direction::Vertical(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(4)
                        .margin(0)
                        .scroller_width(4),
                ))
                .width(Fill)
                .height(Fill)
                .into();
        }

        let rows = fills
            .iter()
            .take(50)
            .fold(Column::new().spacing(2), |col, fill| {
                col.push(self.view_trade_history_row(fill, &theme))
            });

        let mut content = column![header].spacing(4);
        if let Some(warning) = warning {
            content = content.push(text(warning).size(11).color(theme.palette().warning));
        }
        let content = content.push(rule::horizontal(1)).push(rows);
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
}

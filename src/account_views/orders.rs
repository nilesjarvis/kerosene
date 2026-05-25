use crate::account::AccountDataSection;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Column, column, container, row, rule, scrollable, text};
use iced::{Element, Fill};

#[path = "orders/row.rs"]
mod order_row;

// ---------------------------------------------------------------------------
// Open Orders Table
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_open_orders(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let can_cancel = !self.wallet_key_input.trim().is_empty();
        let header_txt = |s: &'static str| {
            text(s)
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(Fill)
        };
        let header = row![
            header_txt("Symbol"),
            header_txt("Side"),
            header_txt("Price"),
            header_txt("Size"),
            text("").width(120),
        ]
        .spacing(4);

        let orders: Vec<_> = self
            .account_data
            .as_ref()
            .map(|d| {
                d.open_orders
                    .iter()
                    .filter(|order| !self.symbol_key_is_hidden(&order.coin))
                    .collect()
            })
            .unwrap_or_default();
        let warning = self.account_data.as_ref().and_then(|data| {
            data.completeness
                .section_warning(AccountDataSection::OpenOrders)
        });

        if orders.is_empty() {
            let msg = if let Some(warning) = warning {
                warning
            } else if self.connected_address.is_some() {
                "No open orders".to_string()
            } else {
                "Connect wallet to view orders".to_string()
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

        let rows = orders.iter().fold(Column::new().spacing(2), |col, order| {
            col.push(self.view_open_order_row(order, can_cancel, &theme))
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

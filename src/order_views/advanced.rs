use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::{Column, column, container, row, rule, scrollable, text};
use iced::{Alignment, Element, Fill};

mod components;
mod rows;
mod summary;

use components::stop_all_button;
use rows::{chase_order_row, history_order_row, twap_order_row};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Advanced Orders
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_advanced_orders(&self) -> Element<'_, Message> {
        let theme = self.theme();

        let mut header = row![text("Advanced Orders").size(12).width(Fill)]
            .spacing(8)
            .align_y(Alignment::Center);
        if self.active_advanced_order_count() > 0 {
            header = header.push(stop_all_button());
        }

        let mut rows = Column::new().spacing(4);
        let active_twaps: Vec<_> = self
            .twap_orders
            .values()
            .filter(|twap| !twap.status.is_terminal())
            .collect();
        if self.chase_orders.is_empty() && active_twaps.is_empty() {
            rows = rows.push(
                container(
                    text("No active advanced orders")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                )
                .width(Fill)
                .height(Fill)
                .center(Fill),
            );
        } else {
            for chase in self.chase_orders.values() {
                rows = rows.push(chase_order_row(chase, &theme, self.spinner_phase));
            }
            for twap in active_twaps {
                rows = rows.push(twap_order_row(twap, &theme, self.spinner_phase));
            }
        }
        rows = rows.push(rule::horizontal(1));
        rows = rows.push(
            text("History")
                .size(11)
                .color(theme.extended_palette().background.weak.text),
        );
        if self.advanced_order_history.is_empty() {
            rows = rows.push(
                text("Completed advanced orders will appear here")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            );
        } else {
            let now_ms = Self::now_ms();
            for entry in self.advanced_order_history.iter().take(40) {
                rows = rows.push(history_order_row(entry, &theme, now_ms));
            }
        }

        let content = column![
            header,
            rule::horizontal(1),
            scrollable(rows)
                .direction(iced::widget::scrollable::Direction::Vertical(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(4)
                        .margin(0)
                        .scroller_width(4)
                ))
                .height(Fill),
        ]
        .spacing(8);

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(8)
            .into()
    }
}

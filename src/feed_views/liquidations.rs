mod controls;
mod footer;
mod rows;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{column, container, scrollable, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_liquidations(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let now_ms = Self::now_ms();

        if self.hydromancer_api_key.trim().is_empty() {
            let empty_state = container(
                text("Add Hydromancer key in Settings > Integrations").color(theme.palette().text),
            )
            .width(Fill)
            .height(Fill)
            .center_x(Fill)
            .center_y(Fill);
            return empty_state.into();
        }

        let content = column![
            self.view_liquidations_top_bar(now_ms),
            self.view_liquidations_header(),
            iced::widget::rule::horizontal(1),
            scrollable(self.view_liquidation_feed_rows(now_ms))
                .direction(iced::widget::scrollable::Direction::Vertical(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(4)
                        .margin(0)
                        .scroller_width(4)
                ))
                .height(Fill),
            iced::widget::rule::horizontal(1),
            self.view_liquidations_bottom_content(now_ms),
        ]
        .spacing(8);

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(12)
            .into()
    }
}

mod controls;
mod layout;
mod rows;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{column, container, responsive, scrollable, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_tracked_trades(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let now_ms = Self::now_ms();
        let labeled_addresses = self.labeled_wallet_addresses();

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

        if labeled_addresses.is_empty() {
            let empty_state =
                container(text("Add wallet labels in Wallet Tracker").color(theme.palette().text))
                    .width(Fill)
                    .height(Fill)
                    .center_x(Fill)
                    .center_y(Fill);
            return empty_state.into();
        }

        container(responsive(move |size| {
            self.view_tracked_trades_sized(now_ms, size.width)
        }))
        .width(Fill)
        .height(Fill)
        .padding(12)
        .into()
    }

    fn view_tracked_trades_sized(&self, now_ms: u64, available_width: f32) -> Element<'_, Message> {
        let row_layout = layout::TrackedTradeRowLayout::from_width(available_width);

        let content = column![
            self.view_tracked_trades_top_bar(now_ms),
            self.view_tracked_trades_header(row_layout),
            iced::widget::rule::horizontal(1),
            scrollable(self.view_tracked_trade_rows(now_ms, row_layout))
                .direction(iced::widget::scrollable::Direction::Vertical(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(4)
                        .margin(0)
                        .scroller_width(4)
                ))
                .height(Fill),
        ]
        .spacing(6);

        container(content)
            .width(Fill)
            .height(Fill)
            .into()
    }
}

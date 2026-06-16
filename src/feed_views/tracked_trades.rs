mod controls;
mod layout;
mod rows;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{column, container, responsive, scrollable};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_tracked_trades(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let now_ms = self.status_bar_now_ms;
        let labeled_addresses = self.labeled_wallet_addresses();
        let tracked_addresses = self.tracked_trade_subscription_addresses();

        if self.hydromancer_api_key.trim().is_empty() {
            return super::feed_empty_state(
                &theme,
                "Add Hydromancer key in Settings > Integrations",
            );
        }

        if labeled_addresses.is_empty() {
            return super::feed_empty_state(&theme, "Add wallet labels in Wallet Tracker");
        }

        if tracked_addresses.is_empty() {
            return super::feed_empty_state(&theme, "All labeled wallets are muted");
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

        container(content).width(Fill).height(Fill).into()
    }
}

mod controls;
mod layout;
mod rows;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{column, container, responsive, scrollable};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_tracked_trades(&self) -> Element<'_, Message> {
        let now_ms = self.status_bar_now_ms;

        if self.hydromancer_api_key.trim().is_empty() {
            return super::feed_empty_state(
                &self.theme(),
                "Add Hydromancer key in Settings > Integrations",
            );
        }

        let labeled_count = self.labeled_wallet_addresses().len();
        if labeled_count == 0 {
            return super::feed_empty_state(&self.theme(), "Add wallet labels in Wallet Tracker");
        }

        let tracked_count = self.tracked_trade_subscription_addresses().len();
        if tracked_count == 0 {
            return super::feed_empty_state(&self.theme(), "All labeled wallets are muted");
        }

        container(responsive(move |size| {
            self.view_tracked_trades_sized(now_ms, size.width, labeled_count, tracked_count)
        }))
        .width(Fill)
        .height(Fill)
        .padding(12)
        .into()
    }

    fn view_tracked_trades_sized(
        &self,
        now_ms: u64,
        available_width: f32,
        labeled_count: usize,
        tracked_count: usize,
    ) -> Element<'_, Message> {
        let row_layout = layout::TrackedTradeRowLayout::from_width(available_width);

        let content = column![
            self.view_tracked_trades_top_bar(now_ms, labeled_count, tracked_count),
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

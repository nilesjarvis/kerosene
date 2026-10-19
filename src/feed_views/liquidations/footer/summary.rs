use crate::app_state::TradingTerminal;
use crate::feed_views::{FEED_SUMMARY_TIMEFRAMES, view_feed_summary};
use crate::message::Message;
use iced::Element;

impl TradingTerminal {
    pub(super) fn view_liquidations_summary(&self, now_ms: u64) -> Element<'_, Message> {
        let theme = self.theme();
        let (buy_color, sell_color) = self.direction_colors(&theme);
        let denomination = self.display_denomination_context();
        view_feed_summary(
            &theme,
            &FEED_SUMMARY_TIMEFRAMES,
            [("L", sell_color), ("S", buy_color)],
            |minutes| self.calculate_liquidation_summary(minutes, now_ms),
            |value| denomination.format_value(value, 0),
        )
    }
}

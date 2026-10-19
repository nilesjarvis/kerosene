use super::{trade_notional_label, trade_quote_currency};
use crate::app_state::TradingTerminal;
use crate::config::OrderBookSymbolModeConfig;
use crate::feed_views::{FEED_SUMMARY_TIMEFRAMES, view_feed_summary};
use crate::message::Message;
use crate::trade_feed_state::TradeFeedInstance;
use iced::widget::{Column, container};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(super) fn view_trade_feed_summary(
        &self,
        feed: &TradeFeedInstance,
        width: f32,
    ) -> Element<'static, Message> {
        let requested = match &feed.mode {
            OrderBookSymbolModeConfig::Active => &self.active_symbol,
            OrderBookSymbolModeConfig::Fixed(key) => key,
        };
        let quote = trade_quote_currency(self.exchange_symbol_for_key(requested));
        let theme = self.theme();
        let columns = if width < 300.0 {
            1
        } else if width < 600.0 {
            2
        } else {
            4
        };
        let mut summary = Column::new().spacing(8).width(Fill);
        for timeframes in FEED_SUMMARY_TIMEFRAMES.chunks(columns) {
            summary = summary.push(view_feed_summary(
                &theme,
                timeframes,
                [
                    ("B", theme.palette().success),
                    ("S", theme.palette().danger),
                ],
                |minutes| feed.calculate_summary(minutes, self.status_bar_now_ms),
                |value| trade_notional_label(value, quote),
            ));
        }
        container(summary).padding([8, 12]).width(Fill).into()
    }
}

#[cfg(test)]
mod tests;

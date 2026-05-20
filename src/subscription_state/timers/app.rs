use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;

use iced::Subscription;

// ---------------------------------------------------------------------------
// App UI Timers
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_ui_timer_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        if !self.toasts.is_empty() || self.nuke_confirmation.is_some() {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(1))
                    .map(|_| Message::TickToastCleanup),
            );
        }

        if self.has_loading_activity() {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(40))
                    .map(|_| Message::SpinnerTick),
            );
        }

        if self.ticker_tape_enabled && !self.favourite_symbols.is_empty() {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(40))
                    .map(|_| Message::TickerTapeTick),
            );
        }

        subs.push(
            iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::StatusBarTick),
        );

        let now_ms = Self::now_ms();
        if self
            .charts
            .values()
            .any(|instance| instance.last_price_flash_is_active(now_ms))
        {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(80))
                    .map(|_| Message::ChartPriceFlashTick),
            );
        }

        subs.push(
            iced::time::every(std::time::Duration::from_secs(60 * 15)).map(|_| Message::Tick),
        );

        if self.pane_is_open(|kind| matches!(kind, PaneKind::HypeEtfs)) {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(60 * 5))
                    .map(|_| Message::HypeEtfsRefreshTick),
            );
        }

        if !self.hydromancer_api_key.trim().is_empty()
            && self
                .charts
                .values()
                .any(|instance| instance.macro_indicators.show_funding_rate)
        {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(60 * 5))
                    .map(|_| Message::FundingRefreshTick),
            );
        }
    }
}

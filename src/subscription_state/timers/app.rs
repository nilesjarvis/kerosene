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

        if self.toast_animations_enabled {
            let now = std::time::Instant::now();
            if self.toasts.iter().any(|toast| toast.is_animating(now)) {
                subs.push(
                    iced::time::every(std::time::Duration::from_millis(16))
                        .map(|_| Message::ToastAnimationTick),
                );
            }
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

        if self.journal.chart_reveal_active() {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(16))
                    .map(|_| Message::JournalChartRevealTick),
            );
        }

        if self.screener.window_id.is_some() {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(15))
                    .map(|_| Message::RefreshScreenerHistory),
            );
            subs.push(
                iced::time::every(std::time::Duration::from_secs(60))
                    .map(|_| Message::RefreshScreener),
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

        if self
            .charts
            .values()
            .any(|instance| instance.chart.hud_animation_tick_needed())
        {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(40))
                    .map(|_| Message::ChartHudOrderAnimationTick),
            );
        }

        if self
            .charts
            .values()
            .any(|instance| instance.chart.order_cancel_hover_animation_active())
        {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(16))
                    .map(|_| Message::ChartOrderCancelHoverAnimationTick),
            );
        }

        if self
            .charts
            .values()
            .any(|instance| instance.chart.earnings_marker_hover_animation_active())
        {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(16))
                    .map(|_| Message::ChartEarningsMarkerHoverAnimationTick),
            );
        }

        if self
            .charts
            .values()
            .any(|instance| instance.chart.hud_armed())
        {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(1))
                    .map(|_| Message::ChartHudSafetyTick),
            );
        }

        subs.push(
            iced::time::every(std::time::Duration::from_secs(60 * 15)).map(|_| Message::Tick),
        );

        // Outcome (HIP-4) markets are listed and expire intraday, so the
        // boot-time symbols snapshot goes stale fast; refresh it periodically
        // (this also retries a failed or partial boot load).
        subs.push(
            iced::time::every(std::time::Duration::from_secs(120))
                .map(|_| Message::ExchangeSymbolsRefreshTick),
        );

        if self.pane_is_open(|kind| matches!(kind, PaneKind::HypeEtfs)) {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(60 * 5))
                    .map(|_| Message::HypeEtfsRefreshTick),
            );
        }

        if self.pane_is_open(|kind| matches!(kind, PaneKind::HypeUnstakingQueue)) {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(60 * 5))
                    .map(|_| Message::HypeUnstakingQueueRefreshTick),
            );
        }

        if self.pane_is_open(|kind| matches!(kind, PaneKind::TelegramFeed)) {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(
                    crate::telegram_feed::TELEGRAM_FEED_REFRESH_INTERVAL_SECS,
                ))
                .map(|_| Message::TelegramFeedRefreshTick),
            );
        }

        if self.pane_is_open(|kind| matches!(kind, PaneKind::XFeed(_))) {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(
                    crate::x_feed::X_FEED_REFRESH_INTERVAL_SECS,
                ))
                .map(|_| Message::XFeedRefreshTick),
            );
        }

        // Schwab access tokens are short-lived (~30 minutes); poll so charts and
        // account data keep working without a manual reconnect.
        if self.schwab.has_refresh_credentials() {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(60))
                    .map(|_| Message::SchwabTokenRefreshTick),
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

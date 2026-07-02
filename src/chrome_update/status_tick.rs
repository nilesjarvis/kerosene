use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::sound;
use iced::Task;
use std::time::Instant;

impl TradingTerminal {
    pub(super) fn handle_status_bar_tick(&mut self) -> Task<Message> {
        let now = Instant::now();
        let now_ms = Self::now_ms();

        self.sync_status_tick_clocks(now, now_ms);
        self.sync_status_tick_models(now, now_ms);

        let mut tasks = vec![
            self.flush_config_save_if_due(now),
            self.stop_chase_if_limits_reached(now),
            self.retry_stopped_chase_cancels(now),
        ];
        tasks.extend(self.queue_chart_asset_context_rest_fetches(now_ms));

        self.drain_sound_status_messages();

        if calendar_retry_due(
            self.is_calendar_open(),
            self.calendar_loading,
            self.calendar_next_retry,
            now,
        ) {
            self.calendar_next_retry = None;
            tasks.push(self.request_calendar_refresh(false));
        }

        Task::batch(tasks)
    }

    fn sync_status_tick_clocks(&mut self, now: Instant, now_ms: u64) {
        self.status_bar_now_ms = now_ms;
        self.status_bar_now = now;
        for instance in self.charts.values_mut() {
            instance.chart.set_clock_now_ms(now_ms);
        }
    }

    fn sync_status_tick_models(&mut self, now: Instant, now_ms: u64) {
        self.sync_chart_display_denominations();
        self.sync_chart_market_reference_prices();
        self.expire_stale_market_asset_contexts(now, now_ms);
        self.expire_pending_order_indicators();
        self.hud_placements.expire(now_ms);
    }

    fn expire_stale_market_asset_contexts(&mut self, now: Instant, now_ms: u64) {
        for instance in self.charts.values_mut() {
            instance.expire_asset_context_if_stale(now_ms);
        }
        for instance in self.order_books.values_mut() {
            instance.expire_asset_context_if_stale(now);
        }
    }

    fn drain_sound_status_messages(&mut self) {
        for status in sound::take_status_messages() {
            self.push_silent_toast(status.message, status.is_error);
        }
    }
}

fn calendar_retry_due(
    is_open: bool,
    is_loading: bool,
    next_retry: Option<Instant>,
    now: Instant,
) -> bool {
    is_open && !is_loading && next_retry.is_some_and(|retry_at| now >= retry_at)
}

#[cfg(test)]
mod tests {
    use super::calendar_retry_due;
    use crate::account::AssetContext;
    use crate::api::{BookLevel, OrderBook};
    use crate::app_state::TradingTerminal;
    use crate::chart_state::ChartInstance;
    use crate::market_state::{
        MARKET_ASSET_CONTEXT_MAX_AGE_MS, OrderBookInstance, OrderBookSymbolMode,
    };
    use crate::timeframe::Timeframe;
    use std::time::{Duration, Instant};

    fn asset_ctx_with_impact(bid: &str, ask: &str) -> AssetContext {
        AssetContext {
            funding: None,
            open_interest: None,
            oracle_px: None,
            mark_px: None,
            mid_px: None,
            prev_day_px: None,
            day_ntl_vlm: None,
            day_base_vlm: None,
            impact_pxs: Some(vec![bid.to_string(), ask.to_string()]),
        }
    }

    fn book() -> OrderBook {
        OrderBook {
            bids: vec![BookLevel { px: 99.0, sz: 1.0 }],
            asks: vec![BookLevel { px: 101.0, sz: 1.0 }],
        }
    }

    #[test]
    fn calendar_retry_requires_open_idle_and_due_retry() {
        let now = Instant::now();
        let due = Some(now - Duration::from_secs(1));
        let future = Some(now + Duration::from_secs(1));

        assert!(calendar_retry_due(true, false, due, now));
        assert!(!calendar_retry_due(false, false, due, now));
        assert!(!calendar_retry_due(true, true, due, now));
        assert!(!calendar_retry_due(true, false, future, now));
        assert!(!calendar_retry_due(true, false, None, now));
    }

    #[test]
    fn status_tick_expires_stale_market_asset_contexts_without_dropping_book_rows() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();
        terminal.order_books.clear();

        let now_ms = TradingTerminal::now_ms();
        let stale_ms = now_ms.saturating_sub(MARKET_ASSET_CONTEXT_MAX_AGE_MS + 1);
        let mut stale_chart = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        stale_chart.set_asset_context_at(Some(asset_ctx_with_impact("99", "101")), stale_ms);
        let mut fresh_chart = ChartInstance::new(2, "ETH".to_string(), Timeframe::H1);
        fresh_chart.set_asset_context_at(Some(asset_ctx_with_impact("199", "201")), now_ms);
        terminal.charts.insert(1, stale_chart);
        terminal.charts.insert(2, fresh_chart);

        let now = Instant::now();
        let stale_instant = now - Duration::from_millis(MARKET_ASSET_CONTEXT_MAX_AGE_MS + 1);
        let mut stale_book = OrderBookInstance::new(1, OrderBookSymbolMode::Active, 1.0);
        stale_book.set_book(book());
        stale_book.asset_ctx = Some(asset_ctx_with_impact("90", "110"));
        stale_book.asset_ctx_updated_at = Some(stale_instant);
        stale_book.spread_history.push_back((stale_instant, 20.0));
        let mut fresh_book = OrderBookInstance::new(2, OrderBookSymbolMode::Active, 1.0);
        fresh_book.set_book(book());
        fresh_book.asset_ctx = Some(asset_ctx_with_impact("190", "210"));
        fresh_book.asset_ctx_updated_at = Some(now);
        fresh_book.spread_history.push_back((now, 20.0));
        terminal.order_books.insert(1, stale_book);
        terminal.order_books.insert(2, fresh_book);

        let _task = terminal.handle_status_bar_tick();

        let stale_chart = &terminal.charts[&1];
        assert!(stale_chart.asset_ctx.is_none());
        assert!(stale_chart.asset_ctx_updated_at_ms.is_none());
        assert!(stale_chart.chart.spread_history.is_empty());
        assert!(terminal.charts[&2].asset_ctx.is_some());
        assert!(!terminal.charts[&2].chart.spread_history.is_empty());

        let stale_book = &terminal.order_books[&1];
        assert!(stale_book.asset_ctx.is_none());
        assert!(stale_book.asset_ctx_updated_at.is_none());
        assert!(!stale_book.spread_history.is_empty());
        assert_eq!(stale_book.best_bid_ask(), (Some(99.0), Some(101.0)));
        assert_eq!(stale_book.book.bids.len(), 1);
        assert_eq!(stale_book.book.asks.len(), 1);

        let fresh_book = &terminal.order_books[&2];
        assert!(fresh_book.asset_ctx.is_some());
        assert_eq!(fresh_book.best_bid_ask(), (Some(190.0), Some(210.0)));
        assert!(!fresh_book.spread_history.is_empty());
    }
}

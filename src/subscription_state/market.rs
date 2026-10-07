use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::read_data_provider::MarketDataSourceContext;
use crate::ws::{
    HydromancerStreamKey, KeyedBookStreamEvent, ws_book_stream_keyed_events,
    ws_hydromancer_book_stream_keyed_events,
};
use iced::Subscription;

mod chart;
mod chase;
mod order_book;
mod position_pnl;
mod positioning_info;
mod spaghetti;
mod twap;

// ---------------------------------------------------------------------------
// Market Subscriptions
// ---------------------------------------------------------------------------

pub(in crate::subscription_state::market) fn source_context_for_stream_event(
    mut source_context: crate::read_data_provider::MarketDataSourceContext,
    hydromancer_key_generation: Option<u64>,
) -> crate::read_data_provider::MarketDataSourceContext {
    source_context.hydromancer_key_generation = hydromancer_key_generation;
    source_context
}

impl TradingTerminal {
    pub(super) fn push_market_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        if self.pane_is_open(|kind| matches!(kind, PaneKind::LiveWatchlist(_))) {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(15))
                    .map(|_| Message::LiveWatchlistRefreshTick),
            );
        }

        if self.ticker_tape_enabled {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(60))
                    .map(|_| Message::TickerTapeRefreshTick),
            );
        }

        self.push_chart_market_subscriptions(subs);
        self.push_spaghetti_market_subscriptions(subs);
        self.push_order_book_subscriptions(subs);
        self.push_position_pnl_market_subscriptions(subs);
        self.push_positioning_info_market_subscriptions(subs);
        self.push_chase_market_subscriptions(subs);
        self.push_twap_market_subscriptions(subs);
    }

    /// Keep the consumer's message mapping at the call site: it distinguishes
    /// order-book, Chase, and TWAP subscriptions with otherwise identical inputs.
    fn market_book_subscription(
        &self,
        id: u64,
        symbol: &str,
    ) -> Subscription<(MarketDataSourceContext, KeyedBookStreamEvent)> {
        let sigfigs = self.canonical_l2_book_sigfigs(symbol);
        self.market_book_subscription_at_precision(id, symbol, sigfigs)
    }

    fn market_book_subscription_at_precision(
        &self,
        id: u64,
        symbol: &str,
        sigfigs: crate::ws::L2BookSigfigs,
    ) -> Subscription<(MarketDataSourceContext, KeyedBookStreamEvent)> {
        let symbol = symbol.to_string();
        let stream = if let Some(api_key) = self.hydromancer_read_provider_key() {
            let stream_key =
                HydromancerStreamKey::from_zeroizing(api_key, self.hydromancer_key_generation);
            Subscription::run_with(
                (stream_key, id, symbol, sigfigs),
                ws_hydromancer_book_stream_keyed_events,
            )
        } else {
            Subscription::run_with((id, symbol, sigfigs), ws_book_stream_keyed_events)
        };
        stream.with(self.market_data_source_context())
    }
}

#[cfg(test)]
mod tests;

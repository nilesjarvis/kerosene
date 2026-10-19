use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::trade_feed_state::TradeFeedSource;
use crate::ws::HydromancerStreamKey;
use iced::Subscription;

impl TradingTerminal {
    pub(super) fn push_trade_feed_subscriptions(
        &self,
        subscriptions: &mut Vec<Subscription<Message>>,
    ) {
        for (&id, feed) in &self.trade_feeds.instances {
            if let Some(symbol) = &feed.symbol {
                let subscription = match feed.source {
                    TradeFeedSource::Hyperliquid => Subscription::run_with(
                        (id, feed.generation, symbol.clone()),
                        crate::ws::ws_trade_feed,
                    ),
                    TradeFeedSource::Hydromancer { key_generation } => Subscription::run_with(
                        (
                            HydromancerStreamKey::new(
                                self.hydromancer_api_key.trim(),
                                key_generation,
                            ),
                            id,
                            feed.generation,
                            symbol.clone(),
                        ),
                        crate::ws::ws_hydromancer_trade_feed,
                    ),
                };
                subscriptions.push(subscription.map(|(id, generation, coin, event)| {
                    Message::TradeFeedStream(id, generation, coin, event)
                }));
            }
        }
    }
}

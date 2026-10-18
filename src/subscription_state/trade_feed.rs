use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Subscription;

impl TradingTerminal {
    pub(super) fn push_trade_feed_subscriptions(
        &self,
        subscriptions: &mut Vec<Subscription<Message>>,
    ) {
        for (&id, feed) in &self.trade_feeds.instances {
            if let Some(symbol) = &feed.symbol {
                subscriptions.push(
                    Subscription::run_with(
                        (id, feed.generation, symbol.clone()),
                        crate::ws::ws_trade_feed,
                    )
                    .map(|(id, generation, coin, event)| {
                        Message::TradeFeedStream(id, generation, coin, event)
                    }),
                );
            }
        }
    }
}

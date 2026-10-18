use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::{HydromancerStreamKey, InspectorStreamConfig, ws_book_inspector};
use iced::Subscription;

impl TradingTerminal {
    pub(super) fn push_book_inspector_subscription(&self, subs: &mut Vec<Subscription<Message>>) {
        let Some(state) = &self.book_inspector else {
            return;
        };
        if self.hydromancer_api_key.trim().is_empty() || self.symbol_key_is_hidden(&state.coin) {
            return;
        }
        let generation = self.hydromancer_key_generation;
        subs.push(
            Subscription::run_with(
                InspectorStreamConfig {
                    key: HydromancerStreamKey::from_zeroizing(
                        self.hydromancer_api_key_for_task(),
                        generation,
                    ),
                    window_id: state.window_id,
                    run_id: state.run_id,
                    coin: state.coin.clone(),
                    selection: state.selection_rx.clone(),
                },
                ws_book_inspector,
            )
            .with((state.window_id, state.run_id, generation))
            .map(|((id, run_id, key_generation), event)| {
                Message::BookInspectorEvent(id, run_id, key_generation, event)
            }),
        );
    }
}

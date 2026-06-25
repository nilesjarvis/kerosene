use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::{
    HydromancerStreamKey, SymbolAssetContextStreamEvent, ws_hydromancer_asset_ctx_stream_symbol,
};

use super::source_context_for_stream_event;
use iced::Subscription;

// ---------------------------------------------------------------------------
// Real-Time Position PnL Streams
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::subscription_state::market) fn push_position_pnl_market_subscriptions(
        &self,
        subs: &mut Vec<Subscription<Message>>,
    ) {
        let symbols = self.hydromancer_realtime_position_pnl_symbols();
        if symbols.is_empty() {
            return;
        }

        let stream_key = HydromancerStreamKey::from_zeroizing(
            self.hydromancer_api_key_for_task(),
            self.hydromancer_key_generation,
        );
        let source_context = self.hydromancer_keyed_market_data_source_context();
        for symbol in symbols {
            subs.push(
                Subscription::run_with(
                    (stream_key.clone(), symbol),
                    ws_hydromancer_asset_ctx_stream_symbol,
                )
                .with(source_context)
                .map(position_pnl_asset_ctx_stream_event_message),
            );
        }
    }
}

pub(super) fn position_pnl_asset_ctx_stream_event_message(
    (source_context, event): (
        crate::read_data_provider::MarketDataSourceContext,
        SymbolAssetContextStreamEvent,
    ),
) -> Message {
    match event {
        SymbolAssetContextStreamEvent::Item(symbol, hydromancer_key_generation, ctx) => {
            let source_context =
                source_context_for_stream_event(source_context, hydromancer_key_generation);
            Message::PositionPnlWsAssetCtxUpdate(symbol, source_context, *ctx)
        }
        SymbolAssetContextStreamEvent::Lagged {
            symbol,
            hydromancer_key_generation,
            skipped,
        } => {
            let source_context =
                source_context_for_stream_event(source_context, hydromancer_key_generation);
            Message::PositionPnlWsAssetCtxLagged(symbol, source_context, skipped)
        }
    }
}

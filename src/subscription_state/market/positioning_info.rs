use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::ws::{
    HydromancerStreamKey, SymbolAssetContextStreamEvent, ws_asset_ctx_stream_symbol,
    ws_hydromancer_asset_ctx_stream_symbol,
};

use iced::Subscription;

// ---------------------------------------------------------------------------
// Positioning Info Market Streams
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::subscription_state::market) fn push_positioning_info_market_subscriptions(
        &self,
        subs: &mut Vec<Subscription<Message>>,
    ) {
        if self.hyperdash_api_key.trim().is_empty() {
            return;
        }

        let mut symbols = Vec::new();
        for (_, kind) in self.panes.iter() {
            let PaneKind::PositioningInfo(id) = kind else {
                continue;
            };
            let Some(instance) = self.positioning_infos.get(id) else {
                continue;
            };
            if instance.symbol.is_empty()
                || self.symbol_key_is_hidden(&instance.symbol)
                || self.hyperdash_coin_for_symbol(&instance.symbol).is_none()
                || symbols.iter().any(|symbol| symbol == &instance.symbol)
            {
                continue;
            }
            symbols.push(instance.symbol.clone());
        }

        let hydromancer_key = self.hydromancer_read_provider_key().map(|api_key| {
            HydromancerStreamKey::from_zeroizing(api_key, self.hydromancer_key_generation)
        });
        let source_context = self.market_data_source_context();
        for symbol in symbols {
            if let Some(api_key) = hydromancer_key.clone() {
                subs.push(
                    Subscription::run_with(
                        (api_key, symbol.clone()),
                        ws_hydromancer_asset_ctx_stream_symbol,
                    )
                    .with(source_context)
                    .map(positioning_asset_ctx_stream_event_message),
                );
            } else {
                subs.push(
                    Subscription::run_with((symbol.clone(),), ws_asset_ctx_stream_symbol)
                        .with(source_context)
                        .map(positioning_asset_ctx_stream_event_message),
                );
            }
        }
    }
}

pub(super) fn positioning_asset_ctx_stream_event_message(
    (source_context, event): (
        crate::read_data_provider::MarketDataSourceContext,
        SymbolAssetContextStreamEvent,
    ),
) -> Message {
    match event {
        SymbolAssetContextStreamEvent::Item(symbol, hydromancer_key_generation, ctx) => {
            debug_assert_eq!(
                source_context.hydromancer_key_generation,
                hydromancer_key_generation
            );
            Message::PositioningInfoWsAssetCtxUpdate(symbol, source_context, *ctx)
        }
        SymbolAssetContextStreamEvent::Lagged {
            symbol,
            hydromancer_key_generation,
            skipped,
        } => {
            debug_assert_eq!(
                source_context.hydromancer_key_generation,
                hydromancer_key_generation
            );
            Message::PositioningInfoWsAssetCtxLagged(symbol, source_context, skipped)
        }
    }
}

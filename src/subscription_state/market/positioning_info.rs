use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::ws::ws_asset_ctx_stream_symbol;

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

        for symbol in symbols {
            subs.push(
                Subscription::run_with((symbol.clone(),), ws_asset_ctx_stream_symbol)
                    .map(|(symbol, ctx)| Message::PositioningInfoWsAssetCtxUpdate(symbol, ctx)),
            );
        }
    }
}

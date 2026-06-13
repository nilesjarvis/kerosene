use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::ws::HydromancerStreamKey;
use iced::Subscription;

// ---------------------------------------------------------------------------
// Hydromancer Subscriptions
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_hydromancer_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        let hydromancer_key = self.hydromancer_api_key.trim();
        if hydromancer_key.is_empty() {
            return;
        }
        let hydromancer_key =
            HydromancerStreamKey::new(hydromancer_key, self.hydromancer_key_generation);

        if self
            .panes
            .iter()
            .any(|(_, kind)| matches!(kind, PaneKind::Liquidations))
        {
            let hydromancer_key_generation = self.hydromancer_key_generation;
            let reconnect_nonce = self.liquidations_reconnect_nonce;
            subs.push(
                Subscription::run_with(
                    (hydromancer_key.clone(), self.liquidations_reconnect_nonce),
                    crate::ws::ws_hydromancer_liquidations,
                )
                .map(move |message| Message::WsHydromancerLiquidation {
                    hydromancer_key_generation,
                    reconnect_nonce,
                    message,
                }),
            );
        }

        if self
            .panes
            .iter()
            .any(|(_, kind)| matches!(kind, PaneKind::TrackedTrades))
        {
            let tracked_addresses = self.tracked_trade_subscription_addresses();
            if !tracked_addresses.is_empty() {
                let hydromancer_key_generation = self.hydromancer_key_generation;
                let reconnect_nonce = self.tracked_trades_reconnect_nonce;
                let tracked_addresses_scope =
                    std::sync::Arc::<[String]>::from(tracked_addresses.clone());
                subs.push(
                    Subscription::run_with(
                        (
                            hydromancer_key.clone(),
                            self.tracked_trades_reconnect_nonce,
                            tracked_addresses,
                        ),
                        crate::ws::ws_hydromancer_tracked_trades,
                    )
                    .map(move |message| Message::WsHydromancerTrackedTrades {
                        hydromancer_key_generation,
                        reconnect_nonce,
                        tracked_addresses: tracked_addresses_scope.clone(),
                        message,
                    }),
                );
            }
        }
    }
}

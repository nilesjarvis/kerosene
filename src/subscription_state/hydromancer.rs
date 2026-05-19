use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;
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

        if self
            .panes
            .iter()
            .any(|(_, kind)| matches!(kind, PaneKind::Liquidations))
        {
            subs.push(
                Subscription::run_with(
                    (
                        hydromancer_key.to_string(),
                        self.liquidations_reconnect_nonce,
                    ),
                    crate::ws::ws_hydromancer_liquidations,
                )
                .map(Message::WsHydromancerLiquidation),
            );
        }

        if self
            .panes
            .iter()
            .any(|(_, kind)| matches!(kind, PaneKind::TrackedTrades))
        {
            let tracked_addresses = self.tracked_trade_subscription_addresses();
            if !tracked_addresses.is_empty() {
                subs.push(
                    Subscription::run_with(
                        (
                            hydromancer_key.to_string(),
                            self.tracked_trades_reconnect_nonce,
                            tracked_addresses,
                        ),
                        crate::ws::ws_hydromancer_tracked_trades,
                    )
                    .map(Message::WsHydromancerTrackedTrades),
                );
            }
        }
    }
}

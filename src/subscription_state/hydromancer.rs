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
                .with((hydromancer_key_generation, reconnect_nonce))
                .map(|((hydromancer_key_generation, reconnect_nonce), message)| {
                    Message::WsHydromancerLiquidation {
                        hydromancer_key_generation,
                        reconnect_nonce,
                        message,
                    }
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
                    .with((
                        hydromancer_key_generation,
                        reconnect_nonce,
                        tracked_addresses_scope,
                    ))
                    .map(
                        |(
                            (hydromancer_key_generation, reconnect_nonce, tracked_addresses),
                            message,
                        )| {
                            Message::WsHydromancerTrackedTrades {
                                hydromancer_key_generation,
                                reconnect_nonce,
                                tracked_addresses: tracked_addresses.into(),
                                message,
                            }
                        },
                    ),
                );
            }
        }

        subs.push(Subscription::run_with(
            hydromancer_key,
            crate::ws::ws_hydromancer_api_latency_probe,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hydromancer_api_latency_probe_requires_configured_key() {
        let terminal = TradingTerminal::boot().0;
        let mut subscriptions = Vec::new();

        terminal.push_hydromancer_subscriptions(&mut subscriptions);

        assert!(subscriptions.is_empty());
    }

    #[test]
    fn hydromancer_api_latency_probe_runs_when_key_configured_without_read_provider() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        let mut subscriptions = Vec::new();

        terminal.push_hydromancer_subscriptions(&mut subscriptions);

        assert_eq!(subscriptions.len(), 1);
    }
}

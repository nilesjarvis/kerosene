use super::*;
use crate::ws::HydromancerWsMessage;
use std::sync::Arc;

enum Freshness {
    Unchanged,
    Updated,
    Cleared,
}

#[test]
fn both_hydromancer_feeds_preserve_control_status_and_freshness_rules() {
    for tracked in [false, true] {
        let mut terminal = TradingTerminal::boot().0;
        for (message, expected_status, freshness) in [
            (
                HydromancerWsMessage::Connecting,
                "Connecting",
                Freshness::Unchanged,
            ),
            (
                HydromancerWsMessage::Resuming,
                "Resuming session",
                Freshness::Unchanged,
            ),
            (
                HydromancerWsMessage::Connected,
                "Connected",
                Freshness::Updated,
            ),
            (
                HydromancerWsMessage::Reconnected,
                "Reconnected",
                Freshness::Updated,
            ),
            (
                HydromancerWsMessage::Heartbeat,
                "Current",
                Freshness::Updated,
            ),
            (
                HydromancerWsMessage::Reconnecting {
                    error: "network unavailable".to_string(),
                    retry_delay_secs: 7,
                },
                "Reconnecting in 7s: network unavailable",
                Freshness::Unchanged,
            ),
            (
                HydromancerWsMessage::Disconnected("closed".to_string()),
                "Disconnected: closed",
                Freshness::Cleared,
            ),
            (
                HydromancerWsMessage::Lagged { skipped: 3 },
                "Stream lagged; reconnecting after skipping 3 messages",
                Freshness::Cleared,
            ),
        ] {
            terminal.liquidations_status = "Current".to_string();
            terminal.tracked_trades_status = "Current".to_string();
            terminal.liquidations_last_rx_ms = Some(123);
            terminal.tracked_trades_last_rx_ms = Some(123);
            let before = TradingTerminal::now_ms();
            let (status, last_rx, other_status, other_last_rx) = if tracked {
                let scoped = Message::WsHydromancerTrackedTrades {
                    hydromancer_key_generation: terminal.hydromancer_key_generation,
                    reconnect_nonce: terminal.tracked_trades_reconnect_nonce,
                    tracked_addresses: Arc::<[String]>::from(
                        terminal.tracked_trade_subscription_addresses(),
                    )
                    .into(),
                    message,
                };
                let _ = terminal.update_tracked_trade_feed(scoped);
                (
                    &terminal.tracked_trades_status,
                    terminal.tracked_trades_last_rx_ms,
                    &terminal.liquidations_status,
                    terminal.liquidations_last_rx_ms,
                )
            } else {
                let scoped = Message::WsHydromancerLiquidation {
                    hydromancer_key_generation: terminal.hydromancer_key_generation,
                    reconnect_nonce: terminal.liquidations_reconnect_nonce,
                    message,
                };
                let _ = terminal.update_liquidation_feed(scoped);
                (
                    &terminal.liquidations_status,
                    terminal.liquidations_last_rx_ms,
                    &terminal.tracked_trades_status,
                    terminal.tracked_trades_last_rx_ms,
                )
            };

            assert_eq!(status, expected_status);
            assert_eq!(other_status, "Current");
            assert_eq!(other_last_rx, Some(123));
            match freshness {
                Freshness::Unchanged => assert_eq!(last_rx, Some(123)),
                Freshness::Updated => {
                    let received = last_rx.expect("control message should refresh the timestamp");
                    assert!(received >= before && received <= TradingTerminal::now_ms());
                }
                Freshness::Cleared => assert_eq!(last_rx, None),
            }
        }
    }
}

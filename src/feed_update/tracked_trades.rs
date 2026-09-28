use super::hydromancer_status::apply_hydromancer_control_message;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_tracked_trade_feed(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WsHydromancerTrackedTrades {
                hydromancer_key_generation,
                reconnect_nonce,
                tracked_addresses,
                message,
            } => {
                let current_tracked_addresses = self.tracked_trade_subscription_addresses();
                if !self.hydromancer_key_generation_is_current(hydromancer_key_generation)
                    || reconnect_nonce != self.tracked_trades_reconnect_nonce
                    || tracked_addresses.as_ref() != current_tracked_addresses.as_slice()
                {
                    return Task::none();
                }

                apply_hydromancer_control_message(
                    &message,
                    &mut self.tracked_trades_status,
                    &mut self.tracked_trades_last_rx_ms,
                );

                if let ws::HydromancerWsMessage::TrackedTrade(trade) = message {
                    let trade = Self::normalize_tracked_trade_event(trade);
                    if !current_tracked_addresses.contains(&trade.address) {
                        return Task::none();
                    }
                    if self.wallet_tracker.is_muted(&trade.address) {
                        return Task::none();
                    }
                    if self.symbol_key_is_hidden(&trade.coin) {
                        return Task::none();
                    }
                    self.tracked_trades_last_rx_ms = Some(Self::now_ms());
                    self.tracked_trades_status = "Connected".to_string();
                    if self.remember_tracked_trade_event(&trade) {
                        let alert_row = self
                            .tracked_trade_alerts_enabled
                            .then(|| self.tracked_trade_alert_row_for_event(&trade))
                            .flatten();
                        self.tracked_trades.push_front(trade);
                        if let Some(row) = alert_row {
                            let alert = self.tracked_trade_alert_message_for_row(&row);
                            self.push_tracked_trade_alert(alert);
                        }
                        if self.tracked_trades.len() > 10000 {
                            self.tracked_trades.truncate(10000);
                        }
                    }
                }
            }
            Message::ClearTrackedTrades => {
                self.tracked_trades.clear();
                self.tracked_trade_seen_keys.clear();
                self.tracked_trade_seen_order.clear();
            }
            _ => {}
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet_state::AddressBookEntry;
    use crate::ws::{HydromancerWsMessage, TrackedTradeEvent};

    const TRACKED_ADDRESS: &str = "0x0000000000000000000000000000000000000001";

    fn scoped_tracked_trade_message(
        terminal: &TradingTerminal,
        message: HydromancerWsMessage,
    ) -> Message {
        Message::WsHydromancerTrackedTrades {
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            reconnect_nonce: terminal.tracked_trades_reconnect_nonce,
            tracked_addresses: std::sync::Arc::<[String]>::from(
                terminal.tracked_trade_subscription_addresses(),
            )
            .into(),
            message,
        }
    }

    fn scoped_tracked_trade_message_with(
        hydromancer_key_generation: u64,
        reconnect_nonce: u64,
        tracked_addresses: Vec<String>,
        message: HydromancerWsMessage,
    ) -> Message {
        Message::WsHydromancerTrackedTrades {
            hydromancer_key_generation,
            reconnect_nonce,
            tracked_addresses: std::sync::Arc::<[String]>::from(tracked_addresses).into(),
            message,
        }
    }

    fn add_tracked_address(terminal: &mut TradingTerminal) {
        terminal.address_book.insert(
            TRACKED_ADDRESS.to_string(),
            AddressBookEntry {
                label: "Tracked".to_string(),
                ..Default::default()
            },
        );
    }

    fn tracked_trade() -> TrackedTradeEvent {
        TrackedTradeEvent {
            address: TRACKED_ADDRESS.to_string(),
            coin: "HYPE".to_string(),
            price: 25.0,
            size: 4.0,
            is_buy: true,
            time_ms: TradingTerminal::now_ms(),
            dir: "Open Long".to_string(),
            start_position: None,
            closed_pnl: 0.0,
            fee: 0.1,
            fee_token: "USDC".to_string(),
            tid: Some(1),
            hash: "0xabc".to_string(),
            oid: Some(10),
            tx_index: 1,
        }
    }

    #[test]
    fn hidden_tracked_trade_does_not_refresh_status_or_create_rows() {
        let mut terminal = TradingTerminal::boot().0;
        add_tracked_address(&mut terminal);
        terminal.muted_tickers.insert("HYPE".to_string());
        terminal.tracked_trades_status = "Current".to_string();
        terminal.tracked_trades_last_rx_ms = Some(123);
        let message = scoped_tracked_trade_message(
            &terminal,
            HydromancerWsMessage::TrackedTrade(tracked_trade()),
        );

        let _ = terminal.update_tracked_trade_feed(message);

        assert_eq!(terminal.tracked_trades_status, "Current");
        assert_eq!(terminal.tracked_trades_last_rx_ms, Some(123));
        assert!(terminal.tracked_trades.is_empty());
        assert!(terminal.tracked_trade_seen_keys.is_empty());
        assert!(terminal.toasts.is_empty());
    }

    #[test]
    fn lagged_tracked_trade_stream_marks_stale_without_dropping_rows() {
        let mut terminal = TradingTerminal::boot().0;
        add_tracked_address(&mut terminal);

        let message = scoped_tracked_trade_message(
            &terminal,
            HydromancerWsMessage::TrackedTrade(tracked_trade()),
        );
        let _ = terminal.update_tracked_trade_feed(message);
        assert!(terminal.tracked_trades_last_rx_ms.is_some());
        assert_eq!(terminal.tracked_trades.len(), 1);

        let message =
            scoped_tracked_trade_message(&terminal, HydromancerWsMessage::Lagged { skipped: 5 });
        let _ = terminal.update_tracked_trade_feed(message);

        assert!(terminal.tracked_trades_last_rx_ms.is_none());
        assert_eq!(
            terminal.tracked_trades_status,
            "Stream lagged; reconnecting after skipping 5 messages"
        );
        assert_eq!(terminal.tracked_trades.len(), 1);
    }

    #[test]
    fn tracked_trade_reconnect_status_redacts_sensitive_hydromancer_error_values() {
        let mut terminal = TradingTerminal::boot().0;
        add_tracked_address(&mut terminal);
        let message = scoped_tracked_trade_message(
            &terminal,
            HydromancerWsMessage::Reconnecting {
                error: "failed wss://api.hydromancer.xyz/ws?Token=hydro-secret&sessionId=session-secret&CURSOR=cursor-secret".to_string(),
                retry_delay_secs: 7,
            },
        );

        let _ = terminal.update_tracked_trade_feed(message);

        assert!(terminal.tracked_trades_status.contains("<redacted>"));
        for secret in ["hydro-secret", "session-secret", "cursor-secret"] {
            assert!(
                !terminal.tracked_trades_status.contains(secret),
                "status leaked {secret}: {}",
                terminal.tracked_trades_status
            );
        }
    }

    #[test]
    fn stale_tracked_trade_stream_scope_is_ignored_without_status_or_alerts() {
        let mut terminal = TradingTerminal::boot().0;
        add_tracked_address(&mut terminal);
        terminal.hydromancer_key_generation = 2;
        terminal.tracked_trades_reconnect_nonce = 3;
        terminal.tracked_trade_alerts_enabled = true;
        terminal.tracked_trades_status = "Current".to_string();

        let message = scoped_tracked_trade_message_with(
            1,
            terminal.tracked_trades_reconnect_nonce,
            terminal.tracked_trade_subscription_addresses(),
            HydromancerWsMessage::TrackedTrade(tracked_trade()),
        );
        let _ = terminal.update_tracked_trade_feed(message);
        let message = scoped_tracked_trade_message_with(
            terminal.hydromancer_key_generation,
            2,
            terminal.tracked_trade_subscription_addresses(),
            HydromancerWsMessage::Connected,
        );
        let _ = terminal.update_tracked_trade_feed(message);

        assert_eq!(terminal.tracked_trades_status, "Current");
        assert!(terminal.tracked_trades_last_rx_ms.is_none());
        assert!(terminal.tracked_trades.is_empty());
        assert!(terminal.toasts.is_empty());
    }

    #[test]
    fn tracked_trade_for_removed_address_is_ignored_without_status_or_alerts() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.tracked_trade_alerts_enabled = true;
        terminal.tracked_trades_status = "Current".to_string();

        let message = scoped_tracked_trade_message_with(
            terminal.hydromancer_key_generation,
            terminal.tracked_trades_reconnect_nonce,
            vec![TRACKED_ADDRESS.to_string()],
            HydromancerWsMessage::TrackedTrade(tracked_trade()),
        );
        let _ = terminal.update_tracked_trade_feed(message);

        assert_eq!(terminal.tracked_trades_status, "Current");
        assert!(terminal.tracked_trades_last_rx_ms.is_none());
        assert!(terminal.tracked_trades.is_empty());
        assert!(terminal.toasts.is_empty());
    }
}

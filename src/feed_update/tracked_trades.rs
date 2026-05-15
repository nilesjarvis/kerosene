use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_tracked_trade_feed(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WsHydromancerTrackedTrades(msg) => match msg {
                ws::HydromancerWsMessage::Connecting => {
                    self.tracked_trades_status = "Connecting".to_string();
                }
                ws::HydromancerWsMessage::Resuming => {
                    self.tracked_trades_status = "Resuming session".to_string();
                }
                ws::HydromancerWsMessage::Connected => {
                    self.tracked_trades_last_rx_ms = Some(Self::now_ms());
                    self.tracked_trades_status = "Connected".to_string();
                }
                ws::HydromancerWsMessage::Reconnected => {
                    self.tracked_trades_last_rx_ms = Some(Self::now_ms());
                    self.tracked_trades_status = "Reconnected".to_string();
                }
                ws::HydromancerWsMessage::Heartbeat => {
                    self.tracked_trades_last_rx_ms = Some(Self::now_ms());
                }
                ws::HydromancerWsMessage::Reconnecting {
                    error,
                    retry_delay_secs,
                } => {
                    self.tracked_trades_status =
                        format!("Reconnecting in {retry_delay_secs}s: {error}");
                }
                ws::HydromancerWsMessage::Disconnected(e) => {
                    self.tracked_trades_last_rx_ms = None;
                    self.tracked_trades_status = format!("Disconnected: {e}");
                }
                ws::HydromancerWsMessage::TrackedTrade(trade) => {
                    let trade = Self::normalize_tracked_trade_event(trade);
                    self.tracked_trades_last_rx_ms = Some(Self::now_ms());
                    self.tracked_trades_status = "Connected".to_string();
                    if self.symbol_key_is_hidden(&trade.coin) {
                        return Task::none();
                    }
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
                ws::HydromancerWsMessage::Event(_) => {}
            },
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

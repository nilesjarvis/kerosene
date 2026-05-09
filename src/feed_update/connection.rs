use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws;
use iced::Task;
use zeroize::Zeroize;

impl TradingTerminal {
    pub(super) fn update_feed_connection(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::HydromancerKeyInputChanged(value) => {
                self.hydromancer_key_input.zeroize();
                self.hydromancer_key_input = value.into();
            }
            Message::SaveHydromancerKey => {
                self.hydromancer_api_key.zeroize();
                self.hydromancer_api_key = self.hydromancer_key_input.trim().to_string().into();
                self.liquidations_last_rx_ms = None;
                self.liquidations_reconnect_nonce =
                    self.liquidations_reconnect_nonce.wrapping_add(1);
                self.tracked_trades_last_rx_ms = None;
                self.tracked_trades_reconnect_nonce =
                    self.tracked_trades_reconnect_nonce.wrapping_add(1);
                self.liquidations_status = if self.hydromancer_api_key.trim().is_empty() {
                    "Disconnected".to_string()
                } else {
                    "Connecting...".to_string()
                };
                self.tracked_trades_status = self.liquidations_status.clone();
                self.persist_hydromancer_secret();
                self.persist_config();
                if !self.hydromancer_api_key.trim().is_empty() {
                    ws::reconnect_hydromancer(self.hydromancer_api_key.trim());
                }
            }
            Message::ReconnectLiquidations if !self.hydromancer_api_key.trim().is_empty() => {
                ws::reconnect_hydromancer(self.hydromancer_api_key.trim());
                self.liquidations_last_rx_ms = None;
                self.liquidations_reconnect_nonce =
                    self.liquidations_reconnect_nonce.wrapping_add(1);
                self.liquidations_status = "Connecting...".to_string();
            }
            Message::ReconnectTrackedTrades if !self.hydromancer_api_key.trim().is_empty() => {
                ws::reconnect_hydromancer(self.hydromancer_api_key.trim());
                self.tracked_trades_last_rx_ms = None;
                self.tracked_trades_reconnect_nonce =
                    self.tracked_trades_reconnect_nonce.wrapping_add(1);
                self.tracked_trades_status = "Connecting...".to_string();
            }
            _ => {}
        }

        Task::none()
    }
}

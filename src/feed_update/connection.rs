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
                // Capture the previous trimmed key BEFORE zeroizing the
                // in-memory state so we can evict its process-wide manager.
                // Without this, the spawned task keeps its owned `api_key`
                // String alive for the lifetime of the app even though
                // the user explicitly rotated/cleared the credential.
                let previous_key = self.hydromancer_api_key.trim().to_string();
                self.hydromancer_api_key.zeroize();
                self.hydromancer_api_key = self.hydromancer_key_input.trim().to_string().into();
                if !previous_key.is_empty() && previous_key != self.hydromancer_api_key.trim() {
                    ws::evict_hydromancer_manager(&previous_key);
                }
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
                self.journal.clear_snapshot_cache();
                self.journal.expanded_snapshot_trade_ids.clear();
                if !self.hydromancer_api_key.trim().is_empty() {
                    ws::reconnect_hydromancer(self.hydromancer_api_key.trim());
                }
                let mut tasks = vec![self.refresh_enabled_funding_charts()];
                if self.chart_backfill_source == crate::config::ChartBackfillSource::Hydromancer {
                    tasks.push(self.reload_chart_backfills_for_source_change());
                }
                return Task::batch(tasks);
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

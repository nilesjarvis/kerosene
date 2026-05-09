use crate::app_state::TradingTerminal;
use crate::ws::TrackedTradeEvent;

use super::super::TRACKED_TRADE_DEDUPE_MAX;

// ---------------------------------------------------------------------------
// Tracked Trade Feed State
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn normalize_tracked_trade_event(mut trade: TrackedTradeEvent) -> TrackedTradeEvent {
        trade.address = Self::normalize_wallet_address(&trade.address)
            .unwrap_or_else(|| trade.address.trim().to_string());
        trade
    }

    pub(crate) fn tracked_trade_event_key(trade: &TrackedTradeEvent) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{:.8}:{:.8}:{}",
            trade.address.to_ascii_lowercase(),
            trade.time_ms,
            trade.tx_index,
            trade.tid.map(|v| v.to_string()).unwrap_or_default(),
            trade.hash.to_ascii_lowercase(),
            trade.oid.map(|v| v.to_string()).unwrap_or_default(),
            trade.coin,
            trade.price,
            trade.size,
            if trade.is_buy { "B" } else { "A" }
        )
    }

    pub(crate) fn remember_tracked_trade_event(&mut self, trade: &TrackedTradeEvent) -> bool {
        let key = Self::tracked_trade_event_key(trade);
        if !self.tracked_trade_seen_keys.insert(key.clone()) {
            return false;
        }

        self.tracked_trade_seen_order.push_back(key);
        while self.tracked_trade_seen_order.len() > TRACKED_TRADE_DEDUPE_MAX {
            if let Some(old) = self.tracked_trade_seen_order.pop_front() {
                self.tracked_trade_seen_keys.remove(&old);
            }
        }

        true
    }

    pub(crate) fn refresh_tracked_trades_subscription(&mut self) {
        self.tracked_trades_last_rx_ms = None;
        self.tracked_trades_reconnect_nonce = self.tracked_trades_reconnect_nonce.wrapping_add(1);
        self.tracked_trades_status = if self.hydromancer_api_key.trim().is_empty() {
            "Disconnected".to_string()
        } else {
            "Connecting...".to_string()
        };
    }
}

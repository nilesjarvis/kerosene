use crate::app_state::TradingTerminal;
use crate::helpers;
use iced::{Color, Theme};

use super::HYDROMANCER_STREAM_STALE_AFTER_MS;

// ---------------------------------------------------------------------------
// Hydromancer Feed Status
// ---------------------------------------------------------------------------

impl TradingTerminal {
    fn hydromancer_status_is_live(status: &str) -> bool {
        status.starts_with("Connected") || status.starts_with("Reconnected")
    }

    fn hydromancer_stream_is_stale(status: &str, last_rx_ms: Option<u64>, now_ms: u64) -> bool {
        Self::hydromancer_status_is_live(status)
            && last_rx_ms.is_none_or(|last_rx| {
                now_ms.saturating_sub(last_rx) > HYDROMANCER_STREAM_STALE_AFTER_MS
            })
    }

    fn hydromancer_connection_label_for(
        status: &str,
        last_rx_ms: Option<u64>,
        now_ms: u64,
    ) -> String {
        if Self::hydromancer_stream_is_stale(status, last_rx_ms, now_ms) {
            return last_rx_ms
                .map(|last_rx| format!("Stale {}", helpers::format_relative_time(last_rx, now_ms)))
                .unwrap_or_else(|| "Stale".to_string());
        }

        if Self::hydromancer_status_is_live(status) {
            return last_rx_ms
                .map(|last_rx| format!("Live {}", helpers::format_relative_time(last_rx, now_ms)))
                .unwrap_or_else(|| status.to_string());
        }

        if let Some((prefix, _reason)) = status.split_once(':') {
            return prefix.to_string();
        }

        status.to_string()
    }

    fn hydromancer_connection_detail_for(
        feed: &str,
        status: &str,
        last_rx_ms: Option<u64>,
        now_ms: u64,
    ) -> String {
        let heartbeat = last_rx_ms
            .map(|last_rx| helpers::format_relative_time(last_rx, now_ms))
            .unwrap_or_else(|| "never".to_string());
        let state = if Self::hydromancer_stream_is_stale(status, last_rx_ms, now_ms) {
            "stale"
        } else {
            status
        };
        format!(
            "{feed}: {state} | last heartbeat/event: {heartbeat} | stale after {}s",
            HYDROMANCER_STREAM_STALE_AFTER_MS / 1000
        )
    }

    pub(crate) fn hydromancer_connection_color(label: &str, theme: &Theme) -> Color {
        if label.starts_with("Live") {
            theme.palette().success
        } else if label.starts_with("Connecting")
            || label.starts_with("Resuming")
            || label.starts_with("Reconnecting")
            || label.starts_with("Stale")
        {
            theme.palette().primary
        } else {
            theme.palette().danger
        }
    }

    pub(crate) fn liquidations_connection_label(&self, now_ms: u64) -> String {
        if self.hydromancer_api_key.trim().is_empty() {
            "Disconnected".to_string()
        } else {
            Self::hydromancer_connection_label_for(
                &self.liquidations_status,
                self.liquidations_last_rx_ms,
                now_ms,
            )
        }
    }

    pub(crate) fn liquidations_connection_detail(&self, now_ms: u64) -> String {
        if self.hydromancer_api_key.trim().is_empty() {
            "Liquidations: disconnected | missing Hydromancer API key".to_string()
        } else {
            Self::hydromancer_connection_detail_for(
                "Liquidations",
                &self.liquidations_status,
                self.liquidations_last_rx_ms,
                now_ms,
            )
        }
    }

    pub(crate) fn tracked_trades_connection_label(&self, now_ms: u64) -> String {
        if self.hydromancer_api_key.trim().is_empty() {
            "Disconnected".to_string()
        } else {
            Self::hydromancer_connection_label_for(
                &self.tracked_trades_status,
                self.tracked_trades_last_rx_ms,
                now_ms,
            )
        }
    }

    pub(crate) fn tracked_trades_connection_detail(&self, now_ms: u64) -> String {
        if self.hydromancer_api_key.trim().is_empty() {
            "Wallet Tracker: disconnected | missing Hydromancer API key".to_string()
        } else {
            Self::hydromancer_connection_detail_for(
                "Wallet Tracker",
                &self.tracked_trades_status,
                self.tracked_trades_last_rx_ms,
                now_ms,
            )
        }
    }
}

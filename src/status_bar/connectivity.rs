mod widgets;

use self::widgets::{
    format_bytes_human, status_group_separator, status_tooltip, unlock_credentials_button,
    ws_status_badge,
};

use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::ws;

use iced::widget::{Row, Space, row, text};
use iced::{Color, Fill};

// ---------------------------------------------------------------------------
// Status Connectivity Row
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn status_connectivity_row(&self) -> Row<'_, Message> {
        let theme = self.theme();
        let ws_stats = ws::telemetry_snapshot();
        let now_ms = Self::now_ms();
        let ws_live = ws_stats.open_connections > 0
            && ws_stats.last_rx_ms > 0
            && now_ms.saturating_sub(ws_stats.last_rx_ms) <= 5_000;
        let (ws_label, ws_color) = if ws_live {
            ("WS LIVE", theme.palette().success)
        } else if ws_stats.open_connections > 0 {
            ("WS STALE", theme.palette().primary)
        } else {
            ("WS OFFLINE", theme.palette().danger)
        };
        let version_color = Color {
            a: 0.42,
            ..theme.extended_palette().background.strong.text
        };

        let mut bottom_row = row![
            ws_status_badge(ws_label, ws_color, ws_live, self.spinner_phase),
            helpers::vertical_spacer(),
            text(format!("{} open conn", ws_stats.open_connections))
                .size(10)
                .color(theme.palette().primary),
            helpers::vertical_spacer(),
            status_tooltip(
                format!("RX {}", format_bytes_human(ws_stats.bytes_received)),
                "RX = cumulative data received from exchange WebSocket streams since app launch",
            ),
            helpers::vertical_spacer(),
            status_tooltip(
                format!("TX {}", format_bytes_human(ws_stats.bytes_sent)),
                "TX = cumulative data sent to exchange WebSocket streams since app launch",
            ),
            helpers::vertical_spacer(),
            text(format!(
                "WS: {}",
                if ws_stats.ws_latency_ms > 0 {
                    format!("{}ms", ws_stats.ws_latency_ms)
                } else {
                    "--ms".to_string()
                }
            ))
            .size(10)
            .color(theme.palette().primary),
            helpers::vertical_spacer(),
            text(format!(
                "API: {}",
                if ws_stats.api_latency_ms > 0 {
                    format!("{}ms", ws_stats.api_latency_ms)
                } else {
                    "--ms".to_string()
                }
            ))
            .size(10)
            .color(theme.palette().primary),
            status_group_separator(),
            self.status_clock_row(),
            Space::new().width(Fill),
        ]
        .spacing(8)
        .width(Fill)
        .align_y(iced::Alignment::Center);

        if self.encrypted_credentials_locked() {
            bottom_row = bottom_row
                .push(helpers::vertical_spacer())
                .push(unlock_credentials_button());
        }

        bottom_row = bottom_row.push(
            text(format!("v{}-alpha", env!("CARGO_PKG_VERSION")))
                .size(10)
                .color(version_color),
        );

        bottom_row
    }
}

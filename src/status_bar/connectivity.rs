mod widgets;

use self::widgets::{
    format_bytes_human, status_group_separator, status_tooltip, unlock_credentials_button,
    ws_status_badge,
};

use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::ws;

use iced::widget::{Row, Space, column, responsive, row, text};
use iced::{Alignment, Color, Element, Fill, Length};

const STATUS_BAR_STACK_BREAKPOINT: f32 = 1_180.0;

// ---------------------------------------------------------------------------
// Status Connectivity Row
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn status_connectivity_row(&self) -> Element<'_, Message> {
        responsive(move |size| self.status_connectivity_layout(size.width))
            .height(Length::Shrink)
            .into()
    }

    fn status_connectivity_layout(&self, available_width: f32) -> Element<'_, Message> {
        if available_width < STATUS_BAR_STACK_BREAKPOINT {
            self.status_connectivity_stacked()
        } else {
            self.status_connectivity_wide()
        }
    }

    fn status_connectivity_wide(&self) -> Element<'_, Message> {
        row![
            self.status_stats_row(true),
            Space::new().width(Fill),
            status_group_separator(),
            self.status_right_row(true),
        ]
        .spacing(12)
        .width(Fill)
        .align_y(Alignment::Center)
        .into()
    }

    fn status_connectivity_stacked(&self) -> Element<'_, Message> {
        column![
            self.status_stats_row(false)
                .width(Fill)
                .wrap()
                .vertical_spacing(4),
            self.status_right_row(false)
                .width(Fill)
                .wrap()
                .vertical_spacing(4),
        ]
        .spacing(4)
        .width(Fill)
        .into()
    }

    fn status_stats_row(&self, separated: bool) -> Row<'static, Message> {
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

        let row = row![ws_status_badge(
            ws_label,
            ws_color,
            ws_live,
            self.spinner_phase
        )]
        .spacing(8)
        .align_y(Alignment::Center);

        let row = push_status_gap(row, separated).push(
            text(format!("{} open conn", ws_stats.open_connections))
                .size(10)
                .color(theme.palette().primary),
        );
        let row = push_status_gap(row, separated).push(status_tooltip(
            format!("RX {}", format_bytes_human(ws_stats.bytes_received)),
            "RX = cumulative data received from exchange WebSocket streams since app launch",
        ));
        let row = push_status_gap(row, separated).push(status_tooltip(
            format!("TX {}", format_bytes_human(ws_stats.bytes_sent)),
            "TX = cumulative data sent to exchange WebSocket streams since app launch",
        ));
        let row = push_status_gap(row, separated).push(
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
        );

        push_status_gap(row, separated).push(
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
        )
    }

    fn status_right_row(&self, separated: bool) -> Row<'static, Message> {
        let theme = self.theme();
        let version_color = Color {
            a: 0.42,
            ..theme.extended_palette().background.strong.text
        };
        let mut row = self.status_clock_row(separated);

        if self.encrypted_credentials_locked() {
            row = push_status_gap(row, separated).push(unlock_credentials_button());
        }

        row.push(
            text(format!("v{}-alpha", env!("CARGO_PKG_VERSION")))
                .size(10)
                .color(version_color),
        )
    }
}

fn push_status_gap(row: Row<'static, Message>, separated: bool) -> Row<'static, Message> {
    if separated {
        row.push(helpers::vertical_spacer())
    } else {
        row
    }
}

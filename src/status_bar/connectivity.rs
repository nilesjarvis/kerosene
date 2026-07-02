mod widgets;

use self::widgets::{
    format_bytes_human, status_group_separator, status_tooltip, unlock_credentials_button,
    ws_status_badge,
};

use crate::app_state::TradingTerminal;
use crate::config::ReadDataProvider;
use crate::helpers;
use crate::message::Message;
use crate::ws;

use iced::widget::{Row, Space, column, responsive, row, text};
use iced::{Alignment, Color, Element, Fill, Length};

const STATUS_BAR_STACK_BREAKPOINT: f32 = 1_180.0;
const API_LATENCY_STALE_AFTER_MS: u64 = 90_000;

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
        let now_ms = self.status_bar_now_ms;
        let ws_live = ws_stats.exchange_open_connections > 0
            && ws_stats.exchange_last_rx_ms > 0
            && now_ms.saturating_sub(ws_stats.exchange_last_rx_ms) <= 5_000;
        let (ws_label, ws_color) = if ws_live {
            ("LIVE", theme.palette().success)
        } else if ws_stats.exchange_open_connections > 0 {
            ("EXCH STALE", theme.palette().primary)
        } else {
            ("EXCH OFFLINE", theme.palette().danger)
        };

        let hydromancer_live = ws_stats.hydromancer_open_connections > 0
            && ws_stats.hydromancer_last_rx_ms > 0
            && now_ms.saturating_sub(ws_stats.hydromancer_last_rx_ms) <= 5_000;
        let (hydromancer_label, hydromancer_color) = if hydromancer_live {
            ("HYDRO LIVE", theme.palette().success)
        } else if ws_stats.hydromancer_open_connections > 0 {
            ("HYDRO STALE", theme.palette().primary)
        } else {
            ("HYDRO OFF", theme.extended_palette().background.weak.text)
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
        let row = push_status_gap(row, separated).push(
            text(format!(
                "{} exch / {} hydro",
                ws_stats.exchange_open_connections, ws_stats.hydromancer_open_connections
            ))
            .size(10)
            .color(theme.palette().primary),
        );
        let row = push_status_gap(row, separated)
            .push(text(hydromancer_label).size(10).color(hydromancer_color));
        let row = push_status_gap(row, separated).push(status_tooltip(
            format!("RX {}", format_bytes_human(ws_stats.bytes_received)),
            "RX = cumulative data received from exchange and Hydromancer WebSocket streams since app launch",
        ));
        let row = push_status_gap(row, separated).push(status_tooltip(
            format!("TX {}", format_bytes_human(ws_stats.bytes_sent)),
            "TX = cumulative data sent to exchange and Hydromancer WebSocket streams since app launch",
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

        let row = push_status_gap(row, separated).push(
            text(format_api_latency_label(
                ws_stats.exchange_open_connections,
                ws_stats.api_latency_ms,
                ws_stats.api_last_success_ms,
                now_ms,
            ))
            .size(10)
            .color(theme.palette().primary),
        );

        let row = if should_show_hydromancer_api_latency_label(
            self.read_data_provider,
            self.hydromancer_api_key.trim(),
        ) {
            push_status_gap(row, separated).push(
                text(format_hydromancer_api_latency_label(
                    ws_stats.hydromancer_api_latency_ms,
                    ws_stats.hydromancer_api_last_success_ms,
                    now_ms,
                ))
                .size(10)
                .color(theme.palette().primary),
            )
        } else {
            row
        };

        row
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

fn format_api_latency_label(
    exchange_open_connections: u64,
    api_latency_ms: u64,
    api_last_success_ms: u64,
    now_ms: u64,
) -> String {
    if exchange_open_connections == 0 || api_latency_ms == 0 || api_last_success_ms == 0 {
        return "API: --ms".to_string();
    }

    if now_ms.saturating_sub(api_last_success_ms) > API_LATENCY_STALE_AFTER_MS {
        "API STALE".to_string()
    } else {
        format!("API: {api_latency_ms}ms")
    }
}

fn should_show_hydromancer_api_latency_label(
    provider: ReadDataProvider,
    hydromancer_api_key: &str,
) -> bool {
    provider == ReadDataProvider::Hydromancer || !hydromancer_api_key.trim().is_empty()
}

fn format_hydromancer_api_latency_label(
    api_latency_ms: u64,
    api_last_success_ms: u64,
    now_ms: u64,
) -> String {
    if api_latency_ms == 0 || api_last_success_ms == 0 {
        return "HYDRO: --ms".to_string();
    }

    if now_ms.saturating_sub(api_last_success_ms) > API_LATENCY_STALE_AFTER_MS {
        "HYDRO STALE".to_string()
    } else {
        format!("HYDRO: {api_latency_ms}ms")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        API_LATENCY_STALE_AFTER_MS, format_api_latency_label, format_hydromancer_api_latency_label,
        should_show_hydromancer_api_latency_label,
    };

    #[test]
    fn api_latency_label_shows_fresh_probe_latency() {
        assert_eq!(
            format_api_latency_label(1, 42, 1_000, 1_000 + API_LATENCY_STALE_AFTER_MS),
            "API: 42ms"
        );
    }

    #[test]
    fn api_latency_label_hides_idle_exchange_latency() {
        assert_eq!(format_api_latency_label(0, 42, 1_000, 1_100), "API: --ms");
    }

    #[test]
    fn api_latency_label_hides_missing_probe_latency() {
        assert_eq!(format_api_latency_label(1, 0, 0, 1_100), "API: --ms");
    }

    #[test]
    fn api_latency_label_marks_old_probe_stale() {
        assert_eq!(
            format_api_latency_label(1, 42, 1_000, 1_001 + API_LATENCY_STALE_AFTER_MS),
            "API STALE"
        );
    }

    #[test]
    fn hydromancer_api_latency_label_visibility_matches_provider_or_key() {
        assert!(!should_show_hydromancer_api_latency_label(
            crate::config::ReadDataProvider::Hyperliquid,
            ""
        ));
        assert!(should_show_hydromancer_api_latency_label(
            crate::config::ReadDataProvider::Hydromancer,
            ""
        ));
        assert!(should_show_hydromancer_api_latency_label(
            crate::config::ReadDataProvider::Hyperliquid,
            " hydro-key "
        ));
    }
    #[test]
    fn hydromancer_api_latency_label_shows_fresh_probe_latency() {
        assert_eq!(
            format_hydromancer_api_latency_label(42, 1_000, 1_000 + API_LATENCY_STALE_AFTER_MS),
            "HYDRO: 42ms"
        );
    }

    #[test]
    fn hydromancer_api_latency_label_hides_missing_probe_latency() {
        assert_eq!(
            format_hydromancer_api_latency_label(0, 0, 1_100),
            "HYDRO: --ms"
        );
    }

    #[test]
    fn hydromancer_api_latency_label_marks_old_probe_stale() {
        assert_eq!(
            format_hydromancer_api_latency_label(42, 1_000, 1_001 + API_LATENCY_STALE_AFTER_MS),
            "HYDRO STALE"
        );
    }
}

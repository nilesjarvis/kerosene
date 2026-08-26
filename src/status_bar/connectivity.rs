pub(super) mod widgets;

use self::widgets::{
    format_bytes_human, status_badge, status_element_tooltip, status_group_separator,
    unlock_credentials_button,
};

use crate::app_state::TradingTerminal;
use crate::config::ReadDataProvider;
use crate::message::Message;
use crate::ws::{self, WsTelemetrySnapshot};

use iced::widget::{Row, Space, column, responsive, row};
use iced::{Alignment, Color, Element, Fill, Length, Theme};

const STATUS_BAR_STACK_BREAKPOINT: f32 = 760.0;
const EXCHANGE_STREAM_STALE_AFTER_MS: u64 = 5_000;
const HYDROMANCER_STREAM_STALE_AFTER_MS: u64 = 75_000;
const LATENCY_STALE_AFTER_MS: u64 = 90_000;

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
            self.status_stats_row(),
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
            self.status_stats_row(),
            self.status_right_row(false)
                .width(Fill)
                .wrap()
                .vertical_spacing(4),
        ]
        .spacing(4)
        .width(Fill)
        .into()
    }

    fn status_stats_row(&self) -> Row<'static, Message> {
        let theme = self.theme();
        let stats = ws::telemetry_snapshot();
        let hydromancer_required = self.read_data_provider == ReadDataProvider::Hydromancer;
        let hydromancer_configured = !self.hydromancer_api_key.trim().is_empty();
        let health = aggregate_data_health(&stats, self.status_bar_now_ms, hydromancer_required);
        let (label, color, pulse) = health.presentation(&theme);
        let detail = connectivity_tooltip(
            &stats,
            self.status_bar_now_ms,
            hydromancer_required,
            hydromancer_configured,
        );

        row![status_element_tooltip(
            status_badge(label, color, pulse, self.spinner_phase),
            detail,
        )]
        .align_y(Alignment::Center)
    }

    fn status_right_row(&self, separated: bool) -> Row<'static, Message> {
        let mut row = self.status_clock_row(separated);

        if self.encrypted_credentials_locked() {
            row = row.push(unlock_credentials_button());
        }

        row
    }
}

// ---------------------------------------------------------------------------
// Aggregate health
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DataHealth {
    Healthy,
    Checking,
    Degraded,
    Offline,
}

impl DataHealth {
    fn presentation(self, theme: &Theme) -> (&'static str, Color, bool) {
        match self {
            Self::Healthy => ("DATA HEALTHY", theme.palette().success, true),
            Self::Checking => ("DATA CHECKING", theme.palette().primary, false),
            Self::Degraded => ("DATA DEGRADED", theme.palette().warning, false),
            Self::Offline => ("DATA OFFLINE", theme.palette().danger, false),
        }
    }
}

fn aggregate_data_health(
    stats: &WsTelemetrySnapshot,
    now_ms: u64,
    hydromancer_required: bool,
) -> DataHealth {
    let mut health = stream_health(
        stats.exchange_open_connections,
        stats.exchange_last_rx_ms,
        now_ms,
        EXCHANGE_STREAM_STALE_AFTER_MS,
    );

    if stats.exchange_open_connections > 0 {
        health = health.max(probe_health(
            stats.api_last_attempt_ms,
            stats.api_last_success_ms,
            stats.api_last_attempt_succeeded,
            stats.api_probe_in_flight,
            now_ms,
        ));
    }

    if hydromancer_required || stats.hydromancer_open_connections > 0 {
        health = health.max(stream_health(
            stats.hydromancer_open_connections,
            stats.hydromancer_last_rx_ms,
            now_ms,
            HYDROMANCER_STREAM_STALE_AFTER_MS,
        ));
    }

    if hydromancer_required {
        health = health.max(probe_health(
            stats.hydromancer_api_last_attempt_ms,
            stats.hydromancer_api_last_success_ms,
            stats.hydromancer_api_last_attempt_succeeded,
            stats.hydromancer_api_probe_in_flight,
            now_ms,
        ));
    }

    health
}

fn stream_health(
    open_connections: u64,
    last_rx_ms: u64,
    now_ms: u64,
    stale_after_ms: u64,
) -> DataHealth {
    if open_connections == 0 {
        DataHealth::Offline
    } else if last_rx_ms == 0 || now_ms.saturating_sub(last_rx_ms) > stale_after_ms {
        DataHealth::Degraded
    } else {
        DataHealth::Healthy
    }
}

fn probe_health(
    last_attempt_ms: u64,
    last_success_ms: u64,
    last_attempt_succeeded: bool,
    in_flight: bool,
    now_ms: u64,
) -> DataHealth {
    let has_fresh_success =
        last_success_ms > 0 && now_ms.saturating_sub(last_success_ms) <= LATENCY_STALE_AFTER_MS;

    if in_flight {
        return if has_fresh_success {
            DataHealth::Healthy
        } else {
            DataHealth::Checking
        };
    }

    if last_attempt_ms == 0 {
        return DataHealth::Checking;
    }

    if !last_attempt_succeeded || !has_fresh_success {
        DataHealth::Degraded
    } else {
        DataHealth::Healthy
    }
}

// ---------------------------------------------------------------------------
// Hover diagnostics
// ---------------------------------------------------------------------------

fn connectivity_tooltip(
    stats: &WsTelemetrySnapshot,
    now_ms: u64,
    hydromancer_required: bool,
    hydromancer_configured: bool,
) -> String {
    let exchange_stream = stream_detail(
        stats.exchange_open_connections,
        stats.exchange_last_rx_ms,
        now_ms,
        EXCHANGE_STREAM_STALE_AFTER_MS,
        "Offline",
    );
    let exchange_ws_latency = ws_latency_detail(
        stats.exchange_open_connections,
        stats.ws_latency_ms,
        stats.ws_latency_last_success_ms,
        now_ms,
    );
    let exchange_api = probe_detail(
        stats.api_latency_ms,
        stats.api_last_attempt_ms,
        stats.api_last_success_ms,
        stats.api_last_attempt_succeeded,
        stats.api_probe_in_flight,
        now_ms,
        "Not measured",
    );

    let hydromancer_idle_label = if hydromancer_configured {
        "Idle"
    } else {
        "Not configured"
    };
    let hydromancer_stream = stream_detail(
        stats.hydromancer_open_connections,
        stats.hydromancer_last_rx_ms,
        now_ms,
        HYDROMANCER_STREAM_STALE_AFTER_MS,
        hydromancer_idle_label,
    );
    let hydromancer_api = if hydromancer_configured || hydromancer_required {
        probe_detail(
            stats.hydromancer_api_latency_ms,
            stats.hydromancer_api_last_attempt_ms,
            stats.hydromancer_api_last_success_ms,
            stats.hydromancer_api_last_attempt_succeeded,
            stats.hydromancer_api_probe_in_flight,
            now_ms,
            "Not measured",
        )
    } else {
        "Not configured".to_string()
    };

    format!(
        "Connectivity\n\nHyperliquid\n  Stream  {exchange_stream}\n  WebSocket latency  {exchange_ws_latency}\n  REST API  {exchange_api}\n\nHydromancer\n  Stream  {hydromancer_stream}\n  REST API  {hydromancer_api}\n\nConnections  {} exchange · {} Hydromancer\nTraffic since launch  RX {} · TX {}",
        stats.exchange_open_connections,
        stats.hydromancer_open_connections,
        format_bytes_human(stats.bytes_received),
        format_bytes_human(stats.bytes_sent),
    )
}

fn stream_detail(
    open_connections: u64,
    last_rx_ms: u64,
    now_ms: u64,
    stale_after_ms: u64,
    disconnected_label: &str,
) -> String {
    if open_connections == 0 {
        return disconnected_label.to_string();
    }

    if last_rx_ms == 0 {
        return "Connected · waiting for data".to_string();
    }

    let age_ms = now_ms.saturating_sub(last_rx_ms);
    if age_ms > stale_after_ms {
        format!("Stale · last data {}", age_label(age_ms))
    } else {
        format!("Live · last data {}", age_label(age_ms))
    }
}

fn ws_latency_detail(
    open_connections: u64,
    latency_ms: u64,
    last_success_ms: u64,
    now_ms: u64,
) -> String {
    if open_connections == 0 {
        return "Not measured".to_string();
    }

    if latency_ms == 0 || last_success_ms == 0 {
        return "Waiting for ping".to_string();
    }

    let age_ms = now_ms.saturating_sub(last_success_ms);
    if age_ms > LATENCY_STALE_AFTER_MS {
        format!("Stale · last measured {}", age_label(age_ms))
    } else {
        format!("{latency_ms} ms · measured {}", age_label(age_ms))
    }
}

#[allow(clippy::too_many_arguments)]
fn probe_detail(
    latency_ms: u64,
    last_attempt_ms: u64,
    last_success_ms: u64,
    last_attempt_succeeded: bool,
    in_flight: bool,
    now_ms: u64,
    unmeasured_label: &str,
) -> String {
    if in_flight {
        return if latency_ms > 0 && last_success_ms > 0 {
            format!("{latency_ms} ms · refreshing")
        } else {
            "Checking".to_string()
        };
    }

    if last_attempt_ms == 0 {
        return unmeasured_label.to_string();
    }

    if !last_attempt_succeeded {
        return format!(
            "Unavailable · failed {}",
            age_label(now_ms.saturating_sub(last_attempt_ms))
        );
    }

    if latency_ms == 0 || last_success_ms == 0 {
        return "No successful measurement".to_string();
    }

    let age_ms = now_ms.saturating_sub(last_success_ms);
    if age_ms > LATENCY_STALE_AFTER_MS {
        format!("Stale · last checked {}", age_label(age_ms))
    } else {
        format!("{latency_ms} ms · checked {}", age_label(age_ms))
    }
}

fn age_label(age_ms: u64) -> String {
    let seconds = age_ms / 1_000;
    if seconds < 1 {
        "just now".to_string()
    } else if seconds < 60 {
        format!("{seconds}s ago")
    } else {
        format!("{}m ago", seconds / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::{DataHealth, LATENCY_STALE_AFTER_MS, aggregate_data_health, probe_detail};
    use crate::ws::WsTelemetrySnapshot;

    fn healthy_exchange(now_ms: u64) -> WsTelemetrySnapshot {
        WsTelemetrySnapshot {
            exchange_open_connections: 1,
            exchange_last_rx_ms: now_ms - 1_000,
            ws_latency_ms: 18,
            ws_latency_last_success_ms: now_ms - 2_000,
            api_latency_ms: 42,
            api_last_attempt_ms: now_ms - 3_000,
            api_last_success_ms: now_ms - 3_000,
            api_last_attempt_succeeded: true,
            ..Default::default()
        }
    }

    #[test]
    fn aggregate_is_healthy_for_fresh_exchange_data_and_api() {
        let now_ms = 100_000;
        let stats = healthy_exchange(now_ms);

        assert_eq!(
            aggregate_data_health(&stats, now_ms, false),
            DataHealth::Healthy
        );
    }

    #[test]
    fn aggregate_is_offline_without_exchange_connection() {
        assert_eq!(
            aggregate_data_health(&WsTelemetrySnapshot::default(), 100_000, false),
            DataHealth::Offline
        );
    }

    #[test]
    fn failed_exchange_probe_degrades_an_otherwise_live_connection() {
        let now_ms = 100_000;
        let mut stats = healthy_exchange(now_ms);
        stats.api_last_attempt_succeeded = false;

        assert_eq!(
            aggregate_data_health(&stats, now_ms, false),
            DataHealth::Degraded
        );
    }

    #[test]
    fn idle_optional_hydromancer_does_not_degrade_exchange_health() {
        let now_ms = 100_000;
        let stats = healthy_exchange(now_ms);

        assert_eq!(
            aggregate_data_health(&stats, now_ms, false),
            DataHealth::Healthy
        );
    }

    #[test]
    fn required_hydromancer_without_a_connection_is_offline() {
        let now_ms = 100_000;
        let stats = healthy_exchange(now_ms);

        assert_eq!(
            aggregate_data_health(&stats, now_ms, true),
            DataHealth::Offline
        );
    }

    #[test]
    fn failed_probe_detail_does_not_reuse_an_old_latency() {
        assert_eq!(
            probe_detail(42, 99_000, 80_000, false, false, 100_000, "Not measured"),
            "Unavailable · failed 1s ago"
        );
    }

    #[test]
    fn old_success_is_reported_as_stale() {
        let now_ms = 200_000;
        assert_eq!(
            probe_detail(
                42,
                now_ms - LATENCY_STALE_AFTER_MS - 1,
                now_ms - LATENCY_STALE_AFTER_MS - 1,
                true,
                false,
                now_ms,
                "Not measured",
            ),
            "Stale · last checked 1m ago"
        );
    }
}

pub(crate) use crate::app_time::now_ms;

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// WebSocket Telemetry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default)]
pub struct WsTelemetrySnapshot {
    pub exchange_open_connections: u64,
    pub hydromancer_open_connections: u64,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub exchange_last_rx_ms: u64,
    pub hydromancer_last_rx_ms: u64,
    pub ws_latency_ms: u64,
    pub ws_latency_last_success_ms: u64,
    pub api_latency_ms: u64,
    pub api_last_attempt_ms: u64,
    pub api_last_success_ms: u64,
    pub api_last_attempt_succeeded: bool,
    pub api_probe_in_flight: bool,
    pub hydromancer_api_latency_ms: u64,
    pub hydromancer_api_last_attempt_ms: u64,
    pub hydromancer_api_last_success_ms: u64,
    pub hydromancer_api_last_attempt_succeeded: bool,
    pub hydromancer_api_probe_in_flight: bool,
}

#[derive(Debug, Default)]
struct WsTelemetry {
    exchange_open_connections: AtomicU64,
    hydromancer_open_connections: AtomicU64,
    bytes_received: AtomicU64,
    bytes_sent: AtomicU64,
    exchange_last_rx_ms: AtomicU64,
    hydromancer_last_rx_ms: AtomicU64,
    ws_ping_start_ms: AtomicU64,
    ws_latency_ms: AtomicU64,
    ws_latency_last_success_ms: AtomicU64,
    api_latency_ms: AtomicU64,
    api_last_attempt_ms: AtomicU64,
    api_last_success_ms: AtomicU64,
    api_last_attempt_succeeded: AtomicBool,
    api_probe_in_flight: AtomicBool,
    hydromancer_api_latency_ms: AtomicU64,
    hydromancer_api_last_attempt_ms: AtomicU64,
    hydromancer_api_last_success_ms: AtomicU64,
    hydromancer_api_last_attempt_succeeded: AtomicBool,
    hydromancer_api_probe_in_flight: AtomicBool,
}

static WS_TELEMETRY: OnceLock<WsTelemetry> = OnceLock::new();

fn ws_telemetry() -> &'static WsTelemetry {
    WS_TELEMETRY.get_or_init(WsTelemetry::default)
}

pub(crate) fn telemetry_on_connect() {
    increment_counter(&ws_telemetry().exchange_open_connections, 1);
    ws_telemetry()
        .exchange_last_rx_ms
        .store(0, Ordering::Relaxed);
    ws_telemetry().ws_ping_start_ms.store(0, Ordering::Relaxed);
    ws_telemetry().ws_latency_ms.store(0, Ordering::Relaxed);
    ws_telemetry()
        .ws_latency_last_success_ms
        .store(0, Ordering::Relaxed);
}

pub(crate) fn telemetry_on_disconnect() {
    decrement_open_connections(&ws_telemetry().exchange_open_connections);
}

pub(crate) fn telemetry_on_hydromancer_connect() {
    increment_counter(&ws_telemetry().hydromancer_open_connections, 1);
    ws_telemetry()
        .hydromancer_last_rx_ms
        .store(0, Ordering::Relaxed);
}

pub(crate) fn telemetry_on_hydromancer_disconnect() {
    decrement_open_connections(&ws_telemetry().hydromancer_open_connections);
}

fn decrement_open_connections(counter: &AtomicU64) {
    let mut current = counter.load(Ordering::Relaxed);
    while current > 0 {
        match counter.compare_exchange_weak(
            current,
            current - 1,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

fn increment_counter(counter: &AtomicU64, amount: u64) {
    if amount == 0 {
        return;
    }

    let mut current = counter.load(Ordering::Relaxed);
    while current < u64::MAX {
        let next = current.saturating_add(amount);
        match counter.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

pub(crate) fn telemetry_add_tx(bytes: u64) {
    increment_counter(&ws_telemetry().bytes_sent, bytes);
}

pub(crate) fn telemetry_add_rx(bytes: u64) {
    increment_counter(&ws_telemetry().bytes_received, bytes);
    ws_telemetry()
        .exchange_last_rx_ms
        .store(now_ms(), Ordering::Relaxed);
}

pub(crate) fn telemetry_add_hydromancer_rx(bytes: u64) {
    increment_counter(&ws_telemetry().bytes_received, bytes);
    ws_telemetry()
        .hydromancer_last_rx_ms
        .store(now_ms(), Ordering::Relaxed);
}

pub(crate) fn telemetry_add_hydromancer_tx(bytes: u64) {
    telemetry_add_tx(bytes);
}

pub(super) fn telemetry_mark_ws_ping_start() {
    ws_telemetry()
        .ws_ping_start_ms
        .store(now_ms(), Ordering::Relaxed);
}

pub(super) fn telemetry_update_ws_latency_from_ping_start() {
    let start_ms = ws_telemetry().ws_ping_start_ms.load(Ordering::Relaxed);
    if start_ms > 0 {
        let now_ms = now_ms();
        let latency = now_ms.saturating_sub(start_ms);
        ws_telemetry()
            .ws_latency_ms
            .store(latency, Ordering::Relaxed);
        ws_telemetry()
            .ws_latency_last_success_ms
            .store(now_ms, Ordering::Relaxed);
    }
}

#[cfg(not(test))]
pub(super) fn telemetry_mark_api_attempt() {
    ws_telemetry()
        .api_last_attempt_ms
        .store(now_ms(), Ordering::Relaxed);
    ws_telemetry()
        .api_probe_in_flight
        .store(true, Ordering::Relaxed);
}

pub(super) fn telemetry_update_api_latency(latency: u64) {
    let now_ms = now_ms();
    ws_telemetry()
        .api_latency_ms
        .store(latency, Ordering::Relaxed);
    ws_telemetry()
        .api_last_success_ms
        .store(now_ms, Ordering::Relaxed);
    ws_telemetry()
        .api_last_attempt_succeeded
        .store(true, Ordering::Relaxed);
    ws_telemetry()
        .api_probe_in_flight
        .store(false, Ordering::Relaxed);
}

#[cfg(not(test))]
pub(super) fn telemetry_record_api_failure() {
    ws_telemetry()
        .api_last_attempt_succeeded
        .store(false, Ordering::Relaxed);
    ws_telemetry()
        .api_probe_in_flight
        .store(false, Ordering::Relaxed);
}

pub(super) fn telemetry_mark_hydromancer_api_attempt() {
    ws_telemetry()
        .hydromancer_api_last_attempt_ms
        .store(now_ms(), Ordering::Relaxed);
    ws_telemetry()
        .hydromancer_api_probe_in_flight
        .store(true, Ordering::Relaxed);
}

pub(super) fn telemetry_update_hydromancer_api_latency(latency: u64) {
    let now_ms = now_ms();
    ws_telemetry()
        .hydromancer_api_latency_ms
        .store(latency, Ordering::Relaxed);
    ws_telemetry()
        .hydromancer_api_last_success_ms
        .store(now_ms, Ordering::Relaxed);
    ws_telemetry()
        .hydromancer_api_last_attempt_succeeded
        .store(true, Ordering::Relaxed);
    ws_telemetry()
        .hydromancer_api_probe_in_flight
        .store(false, Ordering::Relaxed);
}

pub(super) fn telemetry_record_hydromancer_api_failure() {
    ws_telemetry()
        .hydromancer_api_last_attempt_succeeded
        .store(false, Ordering::Relaxed);
    ws_telemetry()
        .hydromancer_api_probe_in_flight
        .store(false, Ordering::Relaxed);
}

pub fn telemetry_snapshot() -> WsTelemetrySnapshot {
    let t = ws_telemetry();
    let exchange_open_connections = t.exchange_open_connections.load(Ordering::Relaxed);
    let hydromancer_open_connections = t.hydromancer_open_connections.load(Ordering::Relaxed);
    let exchange_last_rx_ms = t.exchange_last_rx_ms.load(Ordering::Relaxed);
    let hydromancer_last_rx_ms = t.hydromancer_last_rx_ms.load(Ordering::Relaxed);
    WsTelemetrySnapshot {
        exchange_open_connections,
        hydromancer_open_connections,
        bytes_received: t.bytes_received.load(Ordering::Relaxed),
        bytes_sent: t.bytes_sent.load(Ordering::Relaxed),
        exchange_last_rx_ms,
        hydromancer_last_rx_ms,
        ws_latency_ms: t.ws_latency_ms.load(Ordering::Relaxed),
        ws_latency_last_success_ms: t.ws_latency_last_success_ms.load(Ordering::Relaxed),
        api_latency_ms: t.api_latency_ms.load(Ordering::Relaxed),
        api_last_attempt_ms: t.api_last_attempt_ms.load(Ordering::Relaxed),
        api_last_success_ms: t.api_last_success_ms.load(Ordering::Relaxed),
        api_last_attempt_succeeded: t.api_last_attempt_succeeded.load(Ordering::Relaxed),
        api_probe_in_flight: t.api_probe_in_flight.load(Ordering::Relaxed),
        hydromancer_api_latency_ms: t.hydromancer_api_latency_ms.load(Ordering::Relaxed),
        hydromancer_api_last_attempt_ms: t.hydromancer_api_last_attempt_ms.load(Ordering::Relaxed),
        hydromancer_api_last_success_ms: t.hydromancer_api_last_success_ms.load(Ordering::Relaxed),
        hydromancer_api_last_attempt_succeeded: t
            .hydromancer_api_last_attempt_succeeded
            .load(Ordering::Relaxed),
        hydromancer_api_probe_in_flight: t.hydromancer_api_probe_in_flight.load(Ordering::Relaxed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disconnect_counter_does_not_underflow() {
        let counter = AtomicU64::new(0);

        decrement_open_connections(&counter);

        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn disconnect_counter_decrements_when_open() {
        let counter = AtomicU64::new(2);

        decrement_open_connections(&counter);

        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn increment_counter_saturates_at_u64_max() {
        let counter = AtomicU64::new(u64::MAX - 1);

        increment_counter(&counter, 10);

        assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
    }

    #[test]
    fn increment_counter_ignores_zero_amount() {
        let counter = AtomicU64::new(42);

        increment_counter(&counter, 0);

        assert_eq!(counter.load(Ordering::Relaxed), 42);
    }

    #[test]
    fn api_latency_update_records_success_timestamp() {
        let before = now_ms();

        telemetry_update_api_latency(123);

        let snapshot = telemetry_snapshot();
        assert_eq!(snapshot.api_latency_ms, 123);
        assert!(snapshot.api_last_success_ms >= before);
    }

    #[test]
    fn hydromancer_api_latency_update_records_success_timestamp() {
        let before = now_ms();

        telemetry_update_hydromancer_api_latency(77);

        let snapshot = telemetry_snapshot();
        assert_eq!(snapshot.hydromancer_api_latency_ms, 77);
        assert!(snapshot.hydromancer_api_last_success_ms >= before);
    }
}

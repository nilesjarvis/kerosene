pub(crate) use crate::app_time::now_ms;

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// WebSocket Telemetry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default)]
pub struct WsTelemetrySnapshot {
    pub open_connections: u64,
    pub exchange_open_connections: u64,
    pub hydromancer_open_connections: u64,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub exchange_last_rx_ms: u64,
    pub hydromancer_last_rx_ms: u64,
    pub ws_latency_ms: u64,
    pub api_latency_ms: u64,
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
    api_latency_ms: AtomicU64,
}

static WS_TELEMETRY: OnceLock<WsTelemetry> = OnceLock::new();

fn ws_telemetry() -> &'static WsTelemetry {
    WS_TELEMETRY.get_or_init(WsTelemetry::default)
}

pub(crate) fn telemetry_on_connect() {
    ws_telemetry()
        .exchange_open_connections
        .fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn telemetry_on_disconnect() {
    ws_telemetry()
        .exchange_open_connections
        .fetch_sub(1, Ordering::Relaxed);
}

pub(crate) fn telemetry_on_hydromancer_connect() {
    ws_telemetry()
        .hydromancer_open_connections
        .fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn telemetry_on_hydromancer_disconnect() {
    ws_telemetry()
        .hydromancer_open_connections
        .fetch_sub(1, Ordering::Relaxed);
}

pub(crate) fn telemetry_add_tx(bytes: u64) {
    ws_telemetry()
        .bytes_sent
        .fetch_add(bytes, Ordering::Relaxed);
}

pub(crate) fn telemetry_add_rx(bytes: u64) {
    ws_telemetry()
        .bytes_received
        .fetch_add(bytes, Ordering::Relaxed);
    ws_telemetry()
        .exchange_last_rx_ms
        .store(now_ms(), Ordering::Relaxed);
}

pub(crate) fn telemetry_add_hydromancer_rx(bytes: u64) {
    ws_telemetry()
        .bytes_received
        .fetch_add(bytes, Ordering::Relaxed);
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
        let latency = now_ms().saturating_sub(start_ms);
        ws_telemetry()
            .ws_latency_ms
            .store(latency, Ordering::Relaxed);
    }
}

pub(super) fn telemetry_update_api_latency(latency: u64) {
    ws_telemetry()
        .api_latency_ms
        .store(latency, Ordering::Relaxed);
}

pub fn telemetry_snapshot() -> WsTelemetrySnapshot {
    let t = ws_telemetry();
    let exchange_open_connections = t.exchange_open_connections.load(Ordering::Relaxed);
    let hydromancer_open_connections = t.hydromancer_open_connections.load(Ordering::Relaxed);
    let exchange_last_rx_ms = t.exchange_last_rx_ms.load(Ordering::Relaxed);
    let hydromancer_last_rx_ms = t.hydromancer_last_rx_ms.load(Ordering::Relaxed);
    WsTelemetrySnapshot {
        open_connections: exchange_open_connections + hydromancer_open_connections,
        exchange_open_connections,
        hydromancer_open_connections,
        bytes_received: t.bytes_received.load(Ordering::Relaxed),
        bytes_sent: t.bytes_sent.load(Ordering::Relaxed),
        exchange_last_rx_ms,
        hydromancer_last_rx_ms,
        ws_latency_ms: t.ws_latency_ms.load(Ordering::Relaxed),
        api_latency_ms: t.api_latency_ms.load(Ordering::Relaxed),
    }
}

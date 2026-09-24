//! Audit-only instrumentation, copied into a separate source tree by prepare.py.
//! Labels are generated enum variant names, never Debug-formatted messages.
use crate::{app_state::TradingTerminal, message::Message, network_activity};
use iced::{Element, Subscription, Task, window};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

static TIMINGS: LazyLock<Mutex<BTreeMap<&'static str, Timing>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));
static STATE: LazyLock<Mutex<serde_json::Value>> = LazyLock::new(|| Mutex::new(json!({})));

#[derive(Default, serde::Serialize)]
struct Timing {
    count: u64,
    total_us: u64,
    max_us: u64,
    over_16ms: u64,
    first_ts: u64,
    last_ts: u64,
    // Log2 microsecond buckets: fixed memory, percentile upper bounds.
    histogram: [u64; 32],
}

fn record(name: &'static str, started: Instant) {
    let us = started.elapsed().as_micros().min(u64::MAX as u128) as u64;
    let mut timings = TIMINGS.lock().unwrap_or_else(|e| e.into_inner());
    let t = timings.entry(name).or_default();
    t.last_ts = crate::app_time::now_ms();
    if t.count == 0 {
        t.first_ts = t.last_ts;
    }
    t.count += 1;
    t.total_us += us;
    t.max_us = t.max_us.max(us);
    t.over_16ms += u64::from(us > 16_000);
    let bucket = (64 - us.max(1).leading_zeros() as usize).min(31);
    t.histogram[bucket] += 1;
}

pub(crate) fn update(state: &mut TradingTerminal, message: Message) -> Task<Message> {
    let name = message_name(&message);
    let started = Instant::now();
    let task = state.update(message);
    record(name, started);
    // Counters only: no symbols, account details, inputs, or error strings.
    if name == "StatusBarTick" || name == "SymbolsLoaded" || name == "OutcomeVolumesLoaded" {
        *STATE.lock().unwrap_or_else(|e| e.into_inner()) = json!({
            "charts":state.charts.len(), "symbols":state.exchange_symbols.len(),
            "candles":state.charts.values().map(|c| c.chart.candles.len()).sum::<usize>(),
            "chart_fetch_errors":state.charts.values().filter(|c| c.candle_fetch_error.is_some()).count(),
            "outcome_volumes":state.outcome_volumes_24h.len(), "outcome_loading":state.outcome_volumes_loading,
            "outcome_error":state.outcome_volumes_error.is_some(),
            "onboarding":!state.app_onboarding_dismissed, "connected":state.connected_address.is_some(),
            "screener_open":state.screener.window_id.is_some(), "console_open":state.console.window_id.is_some()
        });
    }
    task
}

pub(crate) fn view(state: &TradingTerminal, id: window::Id) -> Element<'_, Message> {
    let started = Instant::now();
    let result = state.view_window(id);
    record("view", started);
    result
}

pub(crate) fn subscription(state: &TradingTerminal) -> Subscription<Message> {
    let started = Instant::now();
    let result = state.subscription();
    record("subscription", started);
    result
}

pub(crate) fn start() {
    if !std::env::args().any(|a| a == "--test") {
        eprintln!("Audit binary requires --test (no saved accounts or credentials)");
        std::process::exit(2);
    }
    let Some(path) = std::env::var_os("KEROSENE_AUDIT_LOG") else {
        eprintln!("Audit binary requires KEROSENE_AUDIT_LOG");
        std::process::exit(2);
    };
    let Ok(mut output) = OpenOptions::new().write(true).create_new(true).open(path) else {
        eprintln!("Cannot create a new audit log");
        std::process::exit(2);
    };
    std::thread::spawn(move || {
        let mut sequence = 0;
        loop {
            let snapshot = network_activity::snapshot();
            let first = snapshot
                .entries
                .front()
                .map(|e| e.sequence)
                .unwrap_or(sequence + 1);
            let lost = first.saturating_sub(sequence + 1);
            let entries: Vec<_> = snapshot.entries.iter().filter(|e| e.sequence > sequence).map(|e| {
                json!({"seq":e.sequence,"ts":e.timestamp_ms,"provider":e.provider.to_string(),
                    "kind":format!("{:?}",e.kind),"operation":e.operation,"method":e.method,
                    "request_id":e.request_id,"elapsed_ms":e.elapsed_ms,"bytes":e.bytes,"proxied":e.proxied})
            }).collect();
            sequence = snapshot.sequence;
            let timings = std::mem::take(&mut *TIMINGS.lock().unwrap_or_else(|e| e.into_inner()));
            let counts = snapshot.totals[0];
            let state = STATE.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let line = json!({"ts":crate::app_time::now_ms(),"lost_entries":lost,"network":entries,"state":state,
                "timings":timings,"totals":{"requests":counts.requests,"finished":counts.finished,
                    "ws_received":counts.ws_received,"ws_bytes":counts.ws_bytes,"errors":counts.errors,
                    "rate_limited":counts.rate_limited}});
            if writeln!(output, "{line}").is_err() || output.flush().is_err() {
                eprintln!("Audit log write failed");
                return;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    });
}

// Capture observes the original allowlisted metadata before transport redirection.
// Fixture mode redirects every observed HTTP request, including exchange writes,
// to a local server which rejects /exchange. No fallback to live transport.
pub(crate) fn redirect_http(request: &mut reqwest::Request) {
    if let Ok(base) = std::env::var("KEROSENE_AUDIT_HTTP") {
        let path = request.url().path().to_owned();
        let url = format!("{base}{path}")
            .parse()
            .expect("local audit server URL");
        *request.url_mut() = url;
    }
}

pub(crate) fn ws_url(default: &str) -> String {
    std::env::var("KEROSENE_AUDIT_WS").unwrap_or_else(|_| default.to_string())
}

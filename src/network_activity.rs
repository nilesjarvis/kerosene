//! Bounded, metadata-only network telemetry. Never retain URLs, bodies or errors.

mod http;
mod websocket;

pub(crate) use http::{HttpRequestExt, execute};
pub(crate) use websocket::{record_ws_frame, record_ws_lifecycle};

use std::collections::VecDeque;
use std::fmt;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

pub(crate) const HISTORY_LIMIT: usize = 2_000;
const WINDOW_SECONDS: u64 = 60;
static ACTIVITY: LazyLock<Mutex<ActivityLog>> =
    LazyLock::new(|| Mutex::new(ActivityLog::default()));

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Provider {
    #[default]
    All,
    Hyperliquid,
    Hydromancer,
    Hyperdash,
    Hypurrscan,
    OpenRouter,
    Telegram,
    X,
    Sec,
    Calendar,
    Unit,
    Local,
    Other,
}

impl Provider {
    pub(crate) const ALL: [Self; 13] = [
        Self::All,
        Self::Hyperliquid,
        Self::Hydromancer,
        Self::Hyperdash,
        Self::Hypurrscan,
        Self::OpenRouter,
        Self::Telegram,
        Self::X,
        Self::Sec,
        Self::Calendar,
        Self::Unit,
        Self::Local,
        Self::Other,
    ];
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::All => "All providers",
            Self::Hyperliquid => "Hyperliquid",
            Self::Hydromancer => "Hydromancer",
            Self::Hyperdash => "HyperDash",
            Self::Hypurrscan => "Hypurrscan",
            Self::OpenRouter => "OpenRouter",
            Self::Telegram => "Telegram",
            Self::X => "X",
            Self::Sec => "SEC",
            Self::Calendar => "Calendar",
            Self::Unit => "Unit",
            Self::Local => "Local API",
            Self::Other => "Other API",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivityKind {
    HttpSend,
    HttpResponse(u16),
    HttpFailed,
    HttpCancelled,
    WsReceive,
    WsReceiveError,
    WsSend,
    WsConnecting,
    WsConnected,
    WsDisconnected,
    WsFailed,
}

impl ActivityKind {
    pub(crate) fn is_error(self) -> bool {
        matches!(
            self,
            Self::HttpResponse(400..) | Self::HttpFailed | Self::WsFailed | Self::WsReceiveError
        )
    }

    pub(crate) fn is_http(self) -> bool {
        matches!(
            self,
            Self::HttpSend | Self::HttpResponse(_) | Self::HttpFailed | Self::HttpCancelled
        )
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::HttpSend => "HTTP →",
            Self::HttpResponse(_) => "HTTP ←",
            Self::HttpFailed => "HTTP error",
            Self::HttpCancelled => "HTTP cancelled",
            Self::WsReceive => "WS ←",
            Self::WsReceiveError => "WS ← error",
            Self::WsSend => "WS →",
            Self::WsConnecting => "WS connecting",
            Self::WsConnected => "WS connected",
            Self::WsDisconnected => "WS closed",
            Self::WsFailed => "WS error",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ActivityEntry {
    pub(crate) sequence: u64,
    pub(crate) timestamp_ms: u64,
    pub(crate) provider: Provider,
    pub(crate) kind: ActivityKind,
    pub(crate) operation: &'static str,
    pub(crate) method: &'static str,
    pub(crate) request_id: Option<u64>,
    pub(crate) elapsed_ms: Option<u64>,
    pub(crate) bytes: Option<u64>,
    pub(crate) proxied: bool,
}

impl ActivityEntry {
    fn new(provider: Provider, kind: ActivityKind, operation: &'static str) -> Self {
        Self {
            sequence: 0,
            timestamp_ms: 0,
            provider,
            kind,
            operation,
            method: "",
            request_id: None,
            elapsed_ms: None,
            bytes: None,
            proxied: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Counts {
    pub(crate) requests: u64,
    pub(crate) finished: u64,
    pub(crate) ws_received: u64,
    pub(crate) ws_bytes: u64,
    pub(crate) errors: u64,
    pub(crate) rate_limited: u64,
}

impl Counts {
    fn record(&mut self, entry: &ActivityEntry) {
        match entry.kind {
            ActivityKind::HttpSend => self.requests += 1,
            ActivityKind::HttpResponse(_)
            | ActivityKind::HttpFailed
            | ActivityKind::HttpCancelled => self.finished += 1,
            ActivityKind::WsReceive | ActivityKind::WsReceiveError => {
                self.ws_received += 1;
                self.ws_bytes += entry.bytes.unwrap_or(0);
            }
            _ => {}
        }
        self.errors += u64::from(entry.kind.is_error());
        self.rate_limited += u64::from(entry.kind == ActivityKind::HttpResponse(429));
    }

    fn add(&mut self, other: Self) {
        self.requests += other.requests;
        self.finished += other.finished;
        self.ws_received += other.ws_received;
        self.ws_bytes += other.ws_bytes;
        self.errors += other.errors;
        self.rate_limited += other.rate_limited;
    }
}

struct Bucket {
    second: u64,
    counts: [Counts; Provider::ALL.len()],
}

struct ActivityLog {
    started: Instant,
    sequence: u64,
    entries: VecDeque<ActivityEntry>,
    buckets: VecDeque<Bucket>,
    totals: [Counts; Provider::ALL.len()],
}

impl Default for ActivityLog {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            sequence: 0,
            entries: VecDeque::new(),
            buckets: VecDeque::new(),
            totals: [Counts::default(); Provider::ALL.len()],
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ActivitySnapshot {
    pub(crate) entries: VecDeque<ActivityEntry>,
    pub(crate) recent: [Counts; Provider::ALL.len()],
    pub(crate) totals: [Counts; Provider::ALL.len()],
    pub(crate) sequence: u64,
}

impl ActivityLog {
    fn prune_buckets(&mut self, second: u64) {
        while self
            .buckets
            .front()
            .is_some_and(|bucket| second.saturating_sub(bucket.second) >= WINDOW_SECONDS)
        {
            self.buckets.pop_front();
        }
    }

    fn record(&mut self, mut entry: ActivityEntry, second: u64, timestamp_ms: u64) -> u64 {
        self.sequence += 1;
        entry.sequence = self.sequence;
        entry.timestamp_ms = timestamp_ms;
        if entry.kind == ActivityKind::HttpSend {
            entry.request_id = Some(entry.sequence);
        }
        self.prune_buckets(second);
        if self
            .buckets
            .back()
            .is_none_or(|bucket| bucket.second != second)
        {
            self.buckets.push_back(Bucket {
                second,
                counts: [Counts::default(); Provider::ALL.len()],
            });
        }
        for provider in [Provider::All, entry.provider] {
            self.totals[provider as usize].record(&entry);
            if let Some(bucket) = self.buckets.back_mut() {
                bucket.counts[provider as usize].record(&entry);
            }
        }
        if self.entries.len() == HISTORY_LIMIT {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
        self.sequence
    }

    fn snapshot(&mut self, second: u64) -> ActivitySnapshot {
        self.prune_buckets(second);
        let mut recent = [Counts::default(); Provider::ALL.len()];
        for bucket in &self.buckets {
            for (count, value) in recent.iter_mut().zip(bucket.counts) {
                count.add(value);
            }
        }
        ActivitySnapshot {
            entries: self.entries.clone(),
            recent,
            totals: self.totals,
            sequence: self.sequence,
        }
    }
}

fn record(entry: ActivityEntry) -> u64 {
    let mut log = ACTIVITY.lock().unwrap_or_else(|error| error.into_inner());
    let second = log.started.elapsed().as_secs();
    log.record(entry, second, crate::app_time::now_ms())
}

pub(crate) fn snapshot() -> ActivitySnapshot {
    let mut log = ACTIVITY.lock().unwrap_or_else(|error| error.into_inner());
    let second = log.started.elapsed().as_secs();
    log.snapshot(second)
}

/// Only known protocol labels may reach telemetry; never return caller-owned text.
fn safe_operation(value: &str) -> &'static str {
    const OPERATIONS: &[&str] = &[
        "ping",
        "pong",
        "allMids",
        "meta",
        "spotMeta",
        "metaAndAssetCtxs",
        "spotMetaAndAssetCtxs",
        "perpDexs",
        "outcomeMeta",
        "outcomeMetaAndAssetCtxs",
        "candleSnapshot",
        "candle",
        "l2Book",
        "trades",
        "activeAssetCtx",
        "activeSpotAssetCtx",
        "activeAssetData",
        "bbo",
        "webData2",
        "clearinghouseState",
        "spotClearinghouseState",
        "userAbstraction",
        "userFees",
        "userFunding",
        "userFills",
        "userFillsByTime",
        "frontendOpenOrders",
        "openOrders",
        "orderStatus",
        "orderUpdates",
        "userEvents",
        "user",
        "notification",
        "userNonFundingLedgerUpdates",
        "portfolio",
        "portfolioState",
        "batchPortfolioStates",
        "fundingHistory",
        "exchangeStatus",
        "allBorrowLendReserveStates",
        "borrowLendUserState",
        "userBorrowLendInterest",
        "order",
        "cancel",
        "cancelByCloid",
        "batchModify",
        "modify",
        "updateLeverage",
        "subscriptionResponse",
        "subscribe",
        "unsubscribe",
        "connected",
        "reconnected",
        "error",
        "liquidations",
        "liquidationFills",
        "trackedTrades",
        "userTwapSliceFills",
        "userTwapHistory",
    ];
    OPERATIONS
        .iter()
        .copied()
        .find(|known| *known == value)
        .unwrap_or("other")
}

#[cfg(test)]
mod tests;

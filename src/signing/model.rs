use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;
use zeroize::Zeroizing;

use super::numbers::{float_to_wire, round_price};

/// Order type for the exchange API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderKind {
    Market,
    Limit,
    Chase,
}

/// Maximum number of consecutive cancel failures before the chase is
/// automatically stopped to prevent an unbounded retry storm.
pub const MAX_CHASE_CANCEL_RETRIES: u32 = 5;
/// Maximum number of successful/attempted reprices before a chase is stopped.
pub const MAX_CHASE_REPRICES: u32 = 1_000;
/// Maximum wall-clock duration for a single chase lifecycle.
pub const MAX_CHASE_DURATION: Duration = Duration::from_secs(15 * 60);
/// Maximum absolute drift from the initial chase price before auto-stop.
pub const MAX_CHASE_DRIFT_FRACTION: f64 = 0.05;
/// Minimum delay between chase reprice requests.
pub const MIN_CHASE_REPRICE_INTERVAL: Duration = Duration::from_secs(1);
/// Additional pause after the exchange reports a rate-limit response.
pub const CHASE_RATE_LIMIT_COOLDOWN: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChasePendingOp {
    Place,
    Modify { oid: u64 },
    Cancel { oid: u64 },
}

/// Client-side chase order state. Continuously reprices a limit order toward
/// execution until fully filled.
#[derive(Clone)]
pub struct ChaseOrder {
    pub id: u64,
    pub coin: String,
    /// Connected wallet address when the chase was started/adopted.
    pub account_address: String,
    /// Agent key captured at chase start. Lifecycle requests must not read the
    /// mutable UI key field after this point, or account/key edits could reprice
    /// a live chase with the wrong trading identity.
    pub agent_key: Zeroizing<String>,
    pub is_buy: bool,
    pub remaining_size: f64,
    pub asset: u32,
    pub sz_decimals: u32,
    pub is_spot: bool,
    pub reduce_only: bool,
    /// OID of the currently resting limit order (None while placing/cancelling).
    pub current_oid: Option<u64>,
    /// Price of the currently resting limit order.
    pub current_price: f64,
    /// Rounded price string currently expected by the exchange.
    pub current_price_wire: String,
    /// Price where the chase started or was adopted.
    pub initial_price: f64,
    /// Time when the chase started or was adopted.
    pub started_at: std::time::Instant,
    /// Number of completed cancel/place replacement cycles.
    pub reprice_count: u32,
    /// The exchange request currently in flight, if any.
    pub pending_op: Option<ChasePendingOp>,
    /// Time of the most recent reprice request, or a future cooldown anchor
    /// after a rate-limit response.
    pub last_reprice_at: Option<std::time::Instant>,
    /// True after the user has requested a stop while a place/cancel request is
    /// still unresolved. The chase keeps its captured key/account context until
    /// the in-flight request lands so a late resting order can be cancelled.
    pub stop_requested: bool,
    /// User-visible reason to show when a requested stop settles.
    pub stop_reason: Option<(String, bool)>,
    /// Number of consecutive cancel failures. Reset on success. The chase
    /// is auto-stopped when this reaches `MAX_CHASE_CANCEL_RETRIES`.
    pub cancel_retries: u32,
    /// Whether the current `current_oid` has been seen in a WS `openOrders`
    /// snapshot. Prevents stale WS snapshots from prematurely terminating
    /// the chase by concluding the order was "filled" when it was actually
    /// just not yet visible in the WS stream.
    pub oid_confirmed: bool,
    /// Set after one WS open-orders snapshot omits the confirmed order. The
    /// chase waits for an account refresh before concluding the order is gone.
    pub missing_open_order_refresh_requested: bool,
}

impl ChaseOrder {
    pub fn rounded_price(&self, price: f64) -> Option<(f64, String)> {
        if !price.is_finite() || price <= 0.0 {
            return None;
        }
        let rounded = round_price(price, self.sz_decimals, self.is_spot);
        if !rounded.is_finite() || rounded <= 0.0 {
            return None;
        }
        Some((rounded, float_to_wire(rounded)))
    }

    pub fn has_pending_op(&self) -> bool {
        self.pending_op.is_some()
    }

    pub fn can_reprice_now(&self, now: std::time::Instant) -> bool {
        self.last_reprice_at
            .is_none_or(|last| now.saturating_duration_since(last) >= MIN_CHASE_REPRICE_INTERVAL)
    }

    pub fn price_moves_toward_fill(&self, next_price: f64) -> bool {
        if !next_price.is_finite() || next_price <= 0.0 {
            return false;
        }
        if !self.current_price.is_finite() || self.current_price <= 0.0 {
            return true;
        }
        if self.is_buy {
            next_price > self.current_price
        } else {
            next_price < self.current_price
        }
    }
}

impl std::fmt::Debug for ChaseOrder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChaseOrder")
            .field("id", &self.id)
            .field("coin", &self.coin)
            .field("account_address", &self.account_address)
            .field("agent_key", &"<redacted>")
            .field("is_buy", &self.is_buy)
            .field("remaining_size", &self.remaining_size)
            .field("asset", &self.asset)
            .field("sz_decimals", &self.sz_decimals)
            .field("is_spot", &self.is_spot)
            .field("reduce_only", &self.reduce_only)
            .field("current_oid", &self.current_oid)
            .field("current_price", &self.current_price)
            .field("current_price_wire", &self.current_price_wire)
            .field("initial_price", &self.initial_price)
            .field("started_at", &self.started_at)
            .field("reprice_count", &self.reprice_count)
            .field("pending_op", &self.pending_op)
            .field("last_reprice_at", &self.last_reprice_at)
            .field("stop_requested", &self.stop_requested)
            .field("stop_reason", &self.stop_reason)
            .field("cancel_retries", &self.cancel_retries)
            .field("oid_confirmed", &self.oid_confirmed)
            .field(
                "missing_open_order_refresh_requested",
                &self.missing_open_order_refresh_requested,
            )
            .finish()
    }
}

/// Response from the exchange API.
#[derive(Debug, Clone, Deserialize)]
pub struct ExchangeResponse {
    pub status: String,
    pub response: Option<ExchangeResponseInner>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExchangeResponseInner {
    #[serde(rename = "type")]
    pub response_type: String,
    pub data: Option<ExchangeResponseData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExchangeResponseData {
    pub statuses: Vec<Value>,
}

impl ExchangeResponse {
    /// Extract a human-readable summary from the response.
    pub fn summary(&self) -> String {
        if self.status != "ok" {
            return format!("Error: status={}", self.status);
        }
        let Some(inner) = &self.response else {
            return "No response body".to_string();
        };
        let Some(data) = &inner.data else {
            return format!("OK ({})", inner.response_type);
        };
        if data.statuses.is_empty() {
            return "OK (no statuses)".to_string();
        }
        data.statuses
            .iter()
            .map(status_summary)
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Extract the order OID from the response.
    pub fn order_oid(&self) -> Option<u64> {
        let st = self.response.as_ref()?.data.as_ref()?.statuses.first()?;
        if let Some(resting) = st.get("resting")
            && let Some(oid) = resting.get("oid").and_then(|v| v.as_u64())
        {
            return Some(oid);
        }
        if let Some(filled) = st.get("filled")
            && let Some(oid) = filled.get("oid").and_then(|v| v.as_u64())
        {
            return Some(oid);
        }
        None
    }

    /// Whether the order was immediately and fully filled.
    pub fn is_fully_filled(&self) -> bool {
        let Some(statuses) = self
            .response
            .as_ref()
            .and_then(|r| r.data.as_ref())
            .map(|d| d.statuses.as_slice())
        else {
            return false;
        };
        !statuses.is_empty()
            && statuses
                .iter()
                .all(|st| st.get("filled").is_some() && st.get("resting").is_none())
            && !self.is_error()
    }

    /// Whether the response indicates an error.
    pub fn is_error(&self) -> bool {
        if self.status != "ok" {
            return true;
        }
        if let Some(inner) = &self.response
            && let Some(data) = &inner.data
            && !data.statuses.is_empty()
        {
            return data
                .statuses
                .iter()
                .any(|status| status.get("error").is_some());
        }
        false
    }
}

fn status_summary(st: &Value) -> String {
    if let Some(err) = st.get("error").and_then(|v| v.as_str()) {
        return format!("Error: {err}");
    }
    if let Some(filled) = st.get("filled") {
        let sz = filled
            .get("totalSz")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let px = filled.get("avgPx").and_then(|v| v.as_str()).unwrap_or("?");
        let oid = filled.get("oid").and_then(|v| v.as_u64()).unwrap_or(0);
        return format!("Filled {sz} @ ${px} (oid {oid})");
    }
    if let Some(resting) = st.get("resting") {
        let oid = resting.get("oid").and_then(|v| v.as_u64()).unwrap_or(0);
        return format!("Resting (oid {oid})");
    }
    if st.as_str() == Some("success") {
        return "Cancelled".to_string();
    }
    format!("{st}")
}

use serde::{Deserialize, Deserializer};
use serde_json::Value;
use sha3::{Digest, Keccak256};
use std::fmt::Write as _;
use std::time::Duration;
use zeroize::Zeroizing;

use super::numbers::{float_to_wire, round_price};

/// Order type selected by the order entry UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderKind {
    Market,
    Limit,
    Chase,
    Twap,
    LimitIoc,
}

impl OrderKind {
    pub(crate) fn config_str(self) -> &'static str {
        match self {
            Self::Market => "Market",
            Self::Limit => "Limit",
            Self::Chase => "Chase",
            Self::Twap => "TWAP",
            Self::LimitIoc => "Limit IOC",
        }
    }

    pub(crate) fn from_config_str(value: &str) -> Self {
        match value {
            "Market" => Self::Market,
            "Chase" => Self::Chase,
            "TWAP" | "Twap" => Self::Twap,
            "Limit IOC" | "LimitIoc" | "IOC" => Self::LimitIoc,
            _ => Self::Limit,
        }
    }
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
/// Cooldown after a retryable chase exchange error, such as a rate limit.
pub const CHASE_RETRY_COOLDOWN: Duration = Duration::from_secs(5);
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
    pub target_size: f64,
    pub filled_size: f64,
    pub remaining_size: f64,
    pub known_oids: Vec<u64>,
    /// Client order id for the currently placed/adopted order when Kerosene
    /// created it. Adopted resting orders may not have one.
    pub current_cloid: Option<String>,
    /// Number of place attempts made by this chase lifecycle. Used to derive
    /// unique cloids for initial and replacement placements.
    pub place_attempt_count: u32,
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
    /// Wall-clock start time for history snapshots. Live chase state is not
    /// persisted or resumed across restarts.
    pub started_at_ms: u64,
    /// Number of attempted cancel/place replacement cycles.
    pub reprice_count: u32,
    /// The exchange request currently in flight, if any.
    pub pending_op: Option<ChasePendingOp>,
    /// Time of the most recent reprice request.
    pub last_reprice_at: Option<std::time::Instant>,
    /// Latest chase book price queued while waiting to cancel, reconcile, or
    /// place the next residual-sized order.
    pub pending_best_price: Option<f64>,
    /// True when the resting order size must be reduced to match known fills,
    /// but the exchange request is waiting for the shared chase request gate.
    pub pending_size_correction: bool,
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
    pub fn record_oid(&mut self, oid: u64) {
        if !self.known_oids.contains(&oid) {
            self.known_oids.push(oid);
        }
    }

    pub fn known_oids_with_current(&self) -> Vec<u64> {
        let mut oids = self.known_oids.clone();
        if let Some(oid) = self.current_oid
            && !oids.contains(&oid)
        {
            oids.push(oid);
        }
        oids
    }

    pub fn residual_size(&self) -> f64 {
        if !self.target_size.is_finite() || self.target_size <= 0.0 {
            return 0.0;
        }
        let filled = if self.filled_size.is_finite() && self.filled_size > 0.0 {
            self.filled_size.min(self.target_size)
        } else {
            0.0
        };
        (self.target_size - filled).max(0.0)
    }

    pub fn set_filled_size(&mut self, filled_size: f64) -> bool {
        if !filled_size.is_finite() || filled_size < 0.0 {
            return false;
        }
        let filled_size = if self.target_size.is_finite() && self.target_size > 0.0 {
            filled_size.min(self.target_size)
        } else {
            filled_size
        };
        if filled_size <= self.filled_size + f64::EPSILON {
            return false;
        }
        self.filled_size = filled_size;
        self.remaining_size = self.remaining_size.min(self.residual_size()).max(0.0);
        true
    }

    pub fn add_filled_size(&mut self, filled_size: f64) -> bool {
        if !filled_size.is_finite() || filled_size <= 0.0 {
            return false;
        }
        self.set_filled_size(self.filled_size + filled_size)
    }

    pub fn sync_open_remaining_size(&mut self, open_size: f64) -> Option<bool> {
        if !open_size.is_finite() || open_size <= 0.0 {
            return None;
        }
        let residual = self.residual_size();
        let oversized = open_size > residual + f64::EPSILON;
        self.remaining_size = open_size.min(residual).max(0.0);
        Some(oversized)
    }

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

pub fn chase_place_cloid(
    account_address: &str,
    chase_id: u64,
    started_at_ms: u64,
    place_attempt: u32,
) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(b"kerosene:chase-place");
    hasher.update(account_address.as_bytes());
    hasher.update(chase_id.to_be_bytes());
    hasher.update(started_at_ms.to_be_bytes());
    hasher.update(place_attempt.to_be_bytes());

    let digest = hasher.finalize();
    let mut cloid = String::with_capacity(34);
    cloid.push_str("0x");
    for byte in digest.iter().take(16) {
        let _ = write!(cloid, "{byte:02x}");
    }
    cloid
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
            .field("target_size", &self.target_size)
            .field("filled_size", &self.filled_size)
            .field("remaining_size", &self.remaining_size)
            .field("known_oids", &self.known_oids)
            .field("current_cloid", &self.current_cloid)
            .field("place_attempt_count", &self.place_attempt_count)
            .field("asset", &self.asset)
            .field("sz_decimals", &self.sz_decimals)
            .field("is_spot", &self.is_spot)
            .field("reduce_only", &self.reduce_only)
            .field("current_oid", &self.current_oid)
            .field("current_price", &self.current_price)
            .field("current_price_wire", &self.current_price_wire)
            .field("initial_price", &self.initial_price)
            .field("started_at", &self.started_at)
            .field("started_at_ms", &self.started_at_ms)
            .field("reprice_count", &self.reprice_count)
            .field("pending_op", &self.pending_op)
            .field("last_reprice_at", &self.last_reprice_at)
            .field("pending_best_price", &self.pending_best_price)
            .field("pending_size_correction", &self.pending_size_correction)
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
#[derive(Debug, Clone)]
pub struct ExchangeResponse {
    pub status: String,
    pub response: Option<ExchangeResponseInner>,
    raw_response: Option<Value>,
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

#[derive(Debug, Deserialize)]
struct ExchangeResponseWire {
    status: String,
    response: Option<Value>,
}

impl<'de> Deserialize<'de> for ExchangeResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ExchangeResponseWire::deserialize(deserializer)?;
        let mut response = None;
        let mut raw_response = None;

        if let Some(raw) = wire.response {
            match serde_json::from_value::<ExchangeResponseInner>(raw.clone()) {
                Ok(inner) => response = Some(inner),
                Err(_) => raw_response = Some(raw),
            }
        }

        Ok(Self {
            status: wire.status,
            response,
            raw_response,
        })
    }
}

impl ExchangeResponse {
    /// Extract a human-readable summary from the response.
    pub fn summary(&self) -> String {
        if self.status != "ok" {
            if let Some(raw) = &self.raw_response {
                return format!("Error: {}", raw_exchange_response_summary(raw));
            }
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

    pub fn filled_total_size(&self) -> Option<f64> {
        let statuses = self
            .response
            .as_ref()
            .and_then(|r| r.data.as_ref())
            .map(|d| d.statuses.as_slice())?;
        let mut total = 0.0;
        for status in statuses {
            let Some(filled) = status.get("filled") else {
                continue;
            };
            let Some(size) = filled
                .get("totalSz")
                .and_then(|v| v.as_str())
                .and_then(|value| value.parse::<f64>().ok())
            else {
                continue;
            };
            if size.is_finite() && size > 0.0 {
                total += size;
            }
        }
        (total.is_finite() && total > 0.0).then_some(total)
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

    /// Whether an order placement response is too ambiguous to continue automation safely.
    pub fn is_ambiguous_order_result(&self) -> bool {
        if self.status != "ok" {
            return false;
        }
        if self.raw_response.is_some() {
            return true;
        }
        let Some(statuses) = self
            .response
            .as_ref()
            .and_then(|inner| inner.data.as_ref())
            .map(|data| data.statuses.as_slice())
        else {
            return true;
        };
        if statuses.is_empty() {
            return true;
        }
        statuses.iter().any(ambiguous_order_status)
    }

    /// Hyperliquid reports this for IOC orders that were marketable from the
    /// client's last book snapshot but found no resting liquidity at match time.
    pub fn is_ioc_no_match(&self) -> bool {
        self.error_messages().iter().any(|message| {
            message
                .to_ascii_lowercase()
                .contains("could not immediately match against any resting orders")
        })
    }

    fn error_messages(&self) -> Vec<String> {
        let mut messages = Vec::new();
        if self.status != "ok" {
            messages.push(self.status.clone());
        }
        if let Some(raw) = &self.raw_response {
            messages.push(raw_exchange_response_summary(raw));
        }
        if let Some(inner) = &self.response
            && let Some(data) = &inner.data
        {
            messages.extend(data.statuses.iter().filter_map(|status| {
                status
                    .get("error")
                    .and_then(|error| error.as_str())
                    .map(ToString::to_string)
            }));
        }
        messages
    }
}

fn ambiguous_order_status(status: &Value) -> bool {
    if status.get("error").is_some() {
        return false;
    }
    if let Some(resting) = status.get("resting") {
        return resting
            .get("oid")
            .and_then(|value| value.as_u64())
            .is_none();
    }
    if let Some(filled) = status.get("filled") {
        return filled
            .get("totalSz")
            .and_then(|value| value.as_str())
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|size| size.is_finite() && *size > 0.0)
            .is_none();
    }
    true
}

fn raw_exchange_response_summary(value: &Value) -> String {
    value
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_else(|| value.to_string())
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

#[cfg(test)]
mod tests {
    use super::OrderKind;

    #[test]
    fn order_kind_config_strings_round_trip_all_variants() {
        for kind in [
            OrderKind::Market,
            OrderKind::Limit,
            OrderKind::Chase,
            OrderKind::Twap,
            OrderKind::LimitIoc,
        ] {
            assert_eq!(OrderKind::from_config_str(kind.config_str()), kind);
        }
    }

    #[test]
    fn order_kind_config_parser_preserves_limit_ioc_aliases() {
        assert_eq!(OrderKind::from_config_str("Limit IOC"), OrderKind::LimitIoc);
        assert_eq!(OrderKind::from_config_str("LimitIoc"), OrderKind::LimitIoc);
        assert_eq!(OrderKind::from_config_str("IOC"), OrderKind::LimitIoc);
    }

    #[test]
    fn order_kind_config_parser_defaults_unknown_values_to_limit() {
        assert_eq!(OrderKind::from_config_str(""), OrderKind::Limit);
        assert_eq!(OrderKind::from_config_str("Unknown"), OrderKind::Limit);
    }
}

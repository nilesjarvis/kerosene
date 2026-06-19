use super::{ChaseLifecycle, MIN_CHASE_REPRICE_INTERVAL};
use crate::signing::CapturedAgentKey;
use crate::signing::numbers::{float_to_wire, round_price};

// ---------------------------------------------------------------------------
// Chase Order State
// ---------------------------------------------------------------------------

const CHASE_ADOPTED_FILL_CUTOFF_GRACE_MS: u64 = 60_000;

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
    pub agent_key: CapturedAgentKey,
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
    /// Per-OID lower bounds for fill reconciliation. Used for adopted resting
    /// orders so fills from before adoption are not credited to the local
    /// chase lifecycle.
    pub fill_cutoff_ms_by_oid: Vec<(u64, u64)>,
    /// Number of attempted cancel/place replacement cycles.
    pub reprice_count: u32,
    /// Explicit lifecycle state. Chase only sends exchange mutations from
    /// states that make the previous exchange/account state unambiguous.
    pub lifecycle: ChaseLifecycle,
    /// Time of the most recent reprice request.
    pub last_reprice_at: Option<std::time::Instant>,
    /// Latest maker price the chase wants to reach after account state is
    /// verified. Book updates may refresh this, but it is not enough by itself
    /// to authorize a place or modify.
    pub desired_price: Option<f64>,
    /// User-visible reason to show when a requested stop settles.
    pub stop_reason: Option<(String, bool)>,
    /// Number of consecutive cancel failures. Reset on success. The chase
    /// is auto-stopped when this reaches `MAX_CHASE_CANCEL_RETRIES`.
    pub cancel_retries: u32,
}

impl ChaseOrder {
    pub fn adopted_fill_cutoff_ms(started_at_ms: u64) -> u64 {
        started_at_ms.saturating_sub(CHASE_ADOPTED_FILL_CUTOFF_GRACE_MS)
    }

    pub fn record_oid(&mut self, oid: u64) {
        if !self.known_oids.contains(&oid) {
            self.known_oids.push(oid);
        }
    }

    pub fn fill_cutoff_ms_for_oid(&self, oid: u64) -> Option<u64> {
        self.fill_cutoff_ms_by_oid
            .iter()
            .find_map(|(cutoff_oid, cutoff_ms)| (*cutoff_oid == oid).then_some(*cutoff_ms))
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

    pub fn tracks_oid(&self, oid: u64) -> bool {
        self.current_oid == Some(oid) || self.known_oids.contains(&oid)
    }

    pub fn has_exchange_identifier(&self) -> bool {
        self.current_oid.is_some() || self.current_cloid.is_some() || !self.known_oids.is_empty()
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
        self.lifecycle.has_exchange_request()
    }

    pub fn needs_account_verification(&self) -> bool {
        matches!(self.lifecycle, ChaseLifecycle::Verifying { .. })
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
            .field("account_address", &"<redacted>")
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
            .field("fill_cutoff_ms_by_oid", &self.fill_cutoff_ms_by_oid)
            .field("reprice_count", &self.reprice_count)
            .field("lifecycle", &self.lifecycle)
            .field("last_reprice_at", &self.last_reprice_at)
            .field("desired_price", &self.desired_price)
            .field("stop_reason", &self.stop_reason)
            .field("cancel_retries", &self.cancel_retries)
            .finish()
    }
}

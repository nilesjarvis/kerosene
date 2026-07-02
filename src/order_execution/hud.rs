use crate::chart_state::{ChartId, ChartSurfaceId};
use crate::order_pending_indicators::PENDING_ORDER_INDICATOR_TTL_MS;
use std::collections::BTreeMap;
use std::fmt;

// ---------------------------------------------------------------------------
// HUD Chart Order Requests
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HudOrderType {
    Limit,
    Market,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HudOrderSide {
    Long,
    Short,
}

impl HudOrderSide {
    pub(crate) fn is_buy(self) -> bool {
        matches!(self, Self::Long)
    }
}

#[derive(Clone)]
pub(crate) struct HudOrderRequest {
    pub(crate) chart_id: ChartId,
    pub(crate) surface_id: ChartSurfaceId,
    pub(crate) symbol_key: String,
    pub(crate) price: f64,
    pub(crate) quantity: String,
    pub(crate) order_type: HudOrderType,
    pub(crate) market_side: HudOrderSide,
    pub(crate) limit_side: Option<HudOrderSide>,
    pub(crate) click_x: f32,
    pub(crate) click_y: f32,
    pub(crate) chart_w: f32,
    pub(crate) chart_h: f32,
}

impl fmt::Debug for HudOrderRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HudOrderRequest")
            .field("chart_id", &self.chart_id)
            .field("surface_id", &self.surface_id)
            .field("symbol_key", &format_args!("<redacted>"))
            .field("price", &format_args!("<redacted>"))
            .field("quantity", &format_args!("<redacted>"))
            .field("order_type", &self.order_type)
            .field("market_side", &self.market_side)
            .field("limit_side", &self.limit_side)
            .field("click_x", &self.click_x)
            .field("click_y", &self.click_y)
            .field("chart_w", &self.chart_w)
            .field("chart_h", &self.chart_h)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// HUD In-Flight Placement Tracker
// ---------------------------------------------------------------------------

/// Upper bound on concurrent in-flight HUD limit placements per account — a
/// backstop against runaway clicking, well under Hyperliquid's ~100 tracked
/// nonces per signer.
pub(crate) const MAX_INFLIGHT_HUD_PLACEMENTS: usize = 8;

/// HUD limit placements survive to the same deadline as their chart
/// indicators; an entry whose result task never returns must not wedge the
/// in-flight cap forever.
const HUD_PLACEMENT_TTL_MS: u64 = PENDING_ORDER_INDICATOR_TTL_MS;

struct InflightHudPlacement {
    account_address: String,
    pending_indicator_id: Option<u64>,
    started_at_ms: u64,
}

impl fmt::Debug for InflightHudPlacement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InflightHudPlacement")
            .field("account_address", &"<redacted>")
            .field("pending_indicator_id", &self.pending_indicator_id)
            .field("started_at_ms", &self.started_at_ms)
            .finish()
    }
}

/// In-flight HUD limit placements, keyed by a per-tracker id that rides the
/// result message. Unlike the global pending-request gate, entries here only
/// serialize against the cap: concurrent HUD limit clicks each get their own
/// entry and release it when their exchange result (or the TTL sweep) lands.
#[derive(Debug, Default)]
pub(crate) struct HudPlacementTracker {
    inflight: BTreeMap<u64, InflightHudPlacement>,
    next_id: u64,
}

impl HudPlacementTracker {
    pub(crate) fn begin(
        &mut self,
        account_address: String,
        pending_indicator_id: Option<u64>,
        now_ms: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.inflight.insert(
            id,
            InflightHudPlacement {
                account_address,
                pending_indicator_id,
                started_at_ms: now_ms,
            },
        );
        id
    }

    /// Release a placement slot; a stale id (already expired or cleared) is a
    /// no-op so late results never panic or double-free.
    pub(crate) fn finish(&mut self, id: u64) -> bool {
        self.inflight.remove(&id).is_some()
    }

    pub(crate) fn count_for_account(&self, account_address: &str) -> usize {
        self.inflight
            .values()
            .filter(|entry| {
                super::order_account_addresses_match(&entry.account_address, account_address)
            })
            .count()
    }

    pub(crate) fn has_any_for_account(&self, account_address: &str) -> bool {
        self.count_for_account(account_address) > 0
    }

    pub(crate) fn contains_indicator(&self, indicator_id: u64) -> bool {
        self.inflight
            .values()
            .any(|entry| entry.pending_indicator_id == Some(indicator_id))
    }

    pub(crate) fn expire(&mut self, now_ms: u64) -> bool {
        let before = self.inflight.len();
        self.inflight
            .retain(|_, entry| now_ms.saturating_sub(entry.started_at_ms) < HUD_PLACEMENT_TTL_MS);
        before != self.inflight.len()
    }

    pub(crate) fn clear(&mut self) {
        self.inflight.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HUD_PLACEMENT_TTL_MS, HudOrderRequest, HudOrderSide, HudOrderType, HudPlacementTracker,
    };
    use crate::chart_state::ChartSurfaceId;

    #[test]
    fn hud_order_request_debug_redacts_order_details() {
        let request = HudOrderRequest {
            chart_id: 1,
            surface_id: ChartSurfaceId::Docked(1),
            symbol_key: "SECRETCOIN".to_string(),
            price: 98765.4321,
            quantity: "quantity-secret".to_string(),
            order_type: HudOrderType::Limit,
            market_side: HudOrderSide::Long,
            limit_side: Some(HudOrderSide::Short),
            click_x: 120.0,
            click_y: 80.0,
            chart_w: 400.0,
            chart_h: 240.0,
        };

        let rendered = format!("{request:?}");

        assert!(rendered.contains("symbol_key: <redacted>"));
        assert!(rendered.contains("price: <redacted>"));
        assert!(rendered.contains("quantity: <redacted>"));
        assert!(rendered.contains("order_type: Limit"));
        for secret in ["SECRETCOIN", "98765.4321", "quantity-secret"] {
            assert!(!rendered.contains(secret), "{secret} leaked in {rendered}");
        }
    }

    const ACCOUNT_A: &str = "0xAbC0000000000000000000000000000000000001";
    const ACCOUNT_B: &str = "0xdef0000000000000000000000000000000000002";

    #[test]
    fn tracker_counts_entries_per_account() {
        let mut tracker = HudPlacementTracker::default();
        let first = tracker.begin(ACCOUNT_A.to_string(), Some(11), 1_000);
        tracker.begin(ACCOUNT_A.to_string(), Some(12), 1_001);
        tracker.begin(ACCOUNT_B.to_string(), None, 1_002);

        assert_eq!(tracker.count_for_account(ACCOUNT_A), 2);
        assert_eq!(tracker.count_for_account(ACCOUNT_B), 1);
        // Case-insensitive account matching, mirroring order account rules.
        assert_eq!(tracker.count_for_account(&ACCOUNT_A.to_lowercase()), 2);

        assert!(tracker.finish(first));
        assert_eq!(tracker.count_for_account(ACCOUNT_A), 1);
        assert!(!tracker.finish(first), "double finish must be a no-op");
    }

    #[test]
    fn tracker_tracks_indicator_ids() {
        let mut tracker = HudPlacementTracker::default();
        let id = tracker.begin(ACCOUNT_A.to_string(), Some(42), 1_000);
        tracker.begin(ACCOUNT_A.to_string(), None, 1_001);

        assert!(tracker.contains_indicator(42));
        assert!(!tracker.contains_indicator(43));

        tracker.finish(id);
        assert!(!tracker.contains_indicator(42));
    }

    #[test]
    fn tracker_expires_stale_entries() {
        let mut tracker = HudPlacementTracker::default();
        let stale = tracker.begin(ACCOUNT_A.to_string(), Some(1), 1_000);
        tracker.begin(ACCOUNT_A.to_string(), Some(2), 2_000);

        assert!(!tracker.expire(1_000 + HUD_PLACEMENT_TTL_MS - 1));
        assert_eq!(tracker.count_for_account(ACCOUNT_A), 2);

        assert!(tracker.expire(1_000 + HUD_PLACEMENT_TTL_MS));
        assert_eq!(tracker.count_for_account(ACCOUNT_A), 1);
        assert!(!tracker.contains_indicator(1));
        assert!(
            !tracker.finish(stale),
            "finish after expiry must be a no-op"
        );
    }

    #[test]
    fn tracker_clear_releases_everything() {
        let mut tracker = HudPlacementTracker::default();
        tracker.begin(ACCOUNT_A.to_string(), Some(1), 1_000);
        tracker.begin(ACCOUNT_B.to_string(), Some(2), 1_000);

        tracker.clear();

        assert!(!tracker.has_any_for_account(ACCOUNT_A));
        assert!(!tracker.has_any_for_account(ACCOUNT_B));
        assert!(!tracker.contains_indicator(1));
    }

    #[test]
    fn tracker_debug_redacts_account_address() {
        let mut tracker = HudPlacementTracker::default();
        tracker.begin(ACCOUNT_A.to_string(), Some(7), 1_000);

        let rendered = format!("{tracker:?}");

        assert!(rendered.contains("<redacted>"));
        assert!(
            !rendered.contains(ACCOUNT_A),
            "address leaked in {rendered}"
        );
    }
}

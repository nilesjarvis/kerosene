use crate::account::{OpenOrder, UserFill};
use crate::app_state::TradingTerminal;
use crate::helpers::{parse_positive_finite_number, values_match_approx};
use crate::order_execution::order_account_addresses_match;

use std::fmt;

// ---------------------------------------------------------------------------
// Chart-Only Pending Order Indicators
// ---------------------------------------------------------------------------

pub(crate) const PENDING_ORDER_INDICATOR_TTL_MS: u64 = 30_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingOrderIndicatorKind {
    Placing,
    MarketPlacing,
    Cancelling,
    Modifying,
}

#[derive(Clone)]
pub(crate) struct PendingOrderIndicator {
    pub(crate) account_address: String,
    pub(crate) symbol: String,
    pub(crate) oid: Option<u64>,
    pub(crate) is_buy: bool,
    pub(crate) size: String,
    pub(crate) price: String,
    pub(crate) kind: PendingOrderIndicatorKind,
    created_at_ms: u64,
}

impl fmt::Debug for PendingOrderIndicator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PendingOrderIndicator")
            .field("account_address", &"<redacted>")
            .field("symbol", &"<redacted>")
            .field("has_oid", &self.oid.is_some())
            .field("is_buy", &self.is_buy)
            .field("size", &"<redacted>")
            .field("price", &"<redacted>")
            .field("kind", &self.kind)
            .field("created_at_ms", &self.created_at_ms)
            .finish()
    }
}

struct PendingOrderIndicatorInput {
    account_address: String,
    symbol: String,
    oid: Option<u64>,
    is_buy: bool,
    size: String,
    price: String,
    kind: PendingOrderIndicatorKind,
}

/// In-flight decoration for an Orders-tab row, derived from the pending
/// indicators when optimistic account updates are enabled.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum OptimisticOrderRowState {
    Cancelling,
    Modifying { price: String },
}

impl fmt::Debug for OptimisticOrderRowState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelling => f.write_str("Cancelling"),
            Self::Modifying { .. } => f
                .debug_struct("Modifying")
                .field("price", &"<redacted>")
                .finish(),
        }
    }
}

/// Net projected position change for one symbol from in-flight market orders.
#[derive(Clone, PartialEq)]
pub(crate) struct ProjectedPositionDelta {
    pub(crate) symbol: String,
    pub(crate) signed_size: f64,
    pub(crate) estimated_price: Option<f64>,
}

impl fmt::Debug for ProjectedPositionDelta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProjectedPositionDelta")
            .field("symbol", &"<redacted>")
            .field("signed_size", &"<redacted>")
            .field(
                "estimated_price",
                &self.estimated_price.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

impl TradingTerminal {
    pub(crate) fn add_pending_order_placement_indicator(
        &mut self,
        account_address: String,
        symbol: String,
        is_buy: bool,
        size: String,
        price: String,
    ) -> Option<u64> {
        self.add_pending_order_indicator(PendingOrderIndicatorInput {
            account_address,
            symbol,
            oid: None,
            is_buy,
            size,
            price,
            kind: PendingOrderIndicatorKind::Placing,
        })
    }

    pub(crate) fn add_pending_market_order_placement_indicator(
        &mut self,
        account_address: String,
        symbol: String,
        is_buy: bool,
        size: String,
        price: String,
    ) -> Option<u64> {
        self.add_pending_order_indicator(PendingOrderIndicatorInput {
            account_address,
            symbol,
            oid: None,
            is_buy,
            size,
            price,
            kind: PendingOrderIndicatorKind::MarketPlacing,
        })
    }

    fn add_pending_order_indicator(&mut self, input: PendingOrderIndicatorInput) -> Option<u64> {
        match input.kind {
            // Placements render provisional rows/lines from these values, so
            // both must be well-formed.
            PendingOrderIndicatorKind::Placing | PendingOrderIndicatorKind::MarketPlacing => {
                parse_positive_finite_number(&input.size)?;
                parse_positive_finite_number(&input.price)?;
            }
            // Only the new target price is rendered and patched into the
            // local snapshot; the size is copied from the live order, which
            // can be "0.0" for position-tied trigger orders.
            PendingOrderIndicatorKind::Modifying => {
                parse_positive_finite_number(&input.price)?;
            }
            // Cancels only decorate an existing order row/line and gate
            // duplicate cancel requests; TP/SL trigger orders legitimately
            // carry sz "0.0" and trigger-market orders limit_px "0".
            PendingOrderIndicatorKind::Cancelling => {}
        }

        let created_at_ms = Self::now_ms();
        let pending_id = self.next_pending_order_indicator_id(created_at_ms);
        self.pending_order_indicators.insert(
            pending_id,
            PendingOrderIndicator {
                account_address: input.account_address,
                symbol: input.symbol,
                oid: input.oid,
                is_buy: input.is_buy,
                size: input.size,
                price: input.price,
                kind: input.kind,
                created_at_ms,
            },
        );
        self.sync_all_chart_orders();
        Some(pending_id)
    }

    pub(crate) fn add_pending_order_cancellation_indicator(
        &mut self,
        account_address: String,
        order: &OpenOrder,
    ) -> Option<u64> {
        let is_buy = open_order_is_buy(&order.side)?;
        self.add_pending_order_indicator(PendingOrderIndicatorInput {
            account_address,
            symbol: order.coin.clone(),
            oid: Some(order.oid),
            is_buy,
            size: order.sz.clone(),
            price: order.limit_px.clone(),
            kind: PendingOrderIndicatorKind::Cancelling,
        })
    }

    pub(crate) fn add_pending_order_modification_indicator(
        &mut self,
        account_address: String,
        order: &OpenOrder,
        new_price: String,
    ) -> Option<u64> {
        let is_buy = open_order_is_buy(&order.side)?;
        self.add_pending_order_indicator(PendingOrderIndicatorInput {
            account_address,
            symbol: order.coin.clone(),
            oid: Some(order.oid),
            is_buy,
            size: order.sz.clone(),
            price: new_price,
            kind: PendingOrderIndicatorKind::Modifying,
        })
    }

    // ---- Optimistic table projections (Settings > Risk, default off) ----

    /// Placement indicators rendered as provisional rows in the Orders tab.
    /// Market orders never rest, so only limit placements are included.
    pub(crate) fn optimistic_open_order_rows(&self) -> Vec<PendingOrderIndicator> {
        if !self.optimistic_account_updates {
            return Vec::new();
        }
        let Some(account_address) = self.connected_order_account_address() else {
            return Vec::new();
        };
        let open_orders = self
            .connected_order_account_snapshot()
            .map(|(_, data)| data.open_orders.as_slice())
            .unwrap_or_default();
        self.pending_order_indicators
            .values()
            .filter(|indicator| {
                indicator.kind == PendingOrderIndicatorKind::Placing
                    && pending_indicator_is_for_account(indicator, &account_address)
                    && !placing_indicator_matches_confirmed_order(indicator, open_orders)
            })
            .cloned()
            .collect()
    }

    /// In-flight decoration for an existing Orders-tab row. A cancel and a
    /// move can be in flight for the same oid at once; the cancel is the
    /// terminal action, so it always wins, and among moves the most recent
    /// target price is the accurate one.
    pub(crate) fn optimistic_open_order_row_state(
        &self,
        oid: u64,
    ) -> Option<OptimisticOrderRowState> {
        if !self.optimistic_account_updates {
            return None;
        }
        let account_address = self.connected_order_account_address()?;
        let mut modifying = None;
        for indicator in self.pending_order_indicators.values() {
            if !pending_indicator_is_for_account(indicator, &account_address)
                || indicator.oid != Some(oid)
            {
                continue;
            }
            match indicator.kind {
                PendingOrderIndicatorKind::Cancelling => {
                    return Some(OptimisticOrderRowState::Cancelling);
                }
                PendingOrderIndicatorKind::Modifying => {
                    modifying = Some(OptimisticOrderRowState::Modifying {
                        price: indicator.price.clone(),
                    });
                }
                PendingOrderIndicatorKind::Placing | PendingOrderIndicatorKind::MarketPlacing => {}
            }
        }
        modifying
    }

    /// Net projected position-size change per symbol from in-flight market
    /// orders (buys positive, sells negative). Limit placements are excluded
    /// because they rest instead of filling, and TWAP/chase manage their own
    /// lifecycles without indicators. Only perp and outcome symbols project:
    /// spot fills land in balances, never in the positions table.
    pub(crate) fn optimistic_position_deltas(&self) -> Vec<ProjectedPositionDelta> {
        if !self.optimistic_account_updates {
            return Vec::new();
        }
        let Some(account_address) = self.connected_order_account_address() else {
            return Vec::new();
        };
        let mut accumulators: Vec<PositionDeltaAccumulator> = Vec::new();
        for indicator in self.pending_order_indicators.values() {
            if indicator.kind != PendingOrderIndicatorKind::MarketPlacing
                || !pending_indicator_is_for_account(indicator, &account_address)
            {
                continue;
            }
            if !self.is_perp_coin(&indicator.symbol) && !self.is_outcome_coin(&indicator.symbol) {
                continue;
            }
            let Some(size) = parse_positive_finite_number(&indicator.size) else {
                continue;
            };
            match accumulators
                .iter_mut()
                .find(|accumulator| accumulator.symbol == indicator.symbol)
            {
                Some(accumulator) => accumulator.add(size, indicator.is_buy, &indicator.price),
                None => {
                    let mut accumulator =
                        PositionDeltaAccumulator::new(indicator.symbol.clone(), indicator.is_buy);
                    accumulator.add(size, indicator.is_buy, &indicator.price);
                    accumulators.push(accumulator);
                }
            }
        }
        accumulators
            .into_iter()
            .map(PositionDeltaAccumulator::finish)
            .collect()
    }

    pub(crate) fn optimistic_position_delta_for_symbol(&self, symbol: &str) -> Option<f64> {
        self.optimistic_position_deltas()
            .into_iter()
            .find(|delta| delta.symbol == symbol)
            .map(|delta| delta.signed_size)
            .filter(|signed_size| signed_size.abs() > f64::EPSILON)
    }

    pub(crate) fn has_pending_cancel_indicator(&self, oid: u64) -> bool {
        let account_address = self.connected_order_account_address();
        self.pending_order_indicators.values().any(|indicator| {
            indicator.kind == PendingOrderIndicatorKind::Cancelling
                && indicator.oid == Some(oid)
                && pending_indicator_is_for_connected_account(indicator, account_address.as_deref())
        })
    }

    pub(crate) fn has_pending_order_indicator_for_connected_account(&self) -> bool {
        let account_address = self.connected_order_account_address();
        self.pending_order_indicators.values().any(|indicator| {
            pending_indicator_is_for_connected_account(indicator, account_address.as_deref())
        })
    }

    /// Indicators don't record which surface created them; concurrent HUD
    /// limit placements are identified by the in-flight tracker holding their
    /// indicator ids, and everything else still gates HUD clicks.
    pub(crate) fn has_non_hud_pending_order_indicator_for_connected_account(&self) -> bool {
        let account_address = self.connected_order_account_address();
        self.pending_order_indicators
            .iter()
            .any(|(pending_id, indicator)| {
                pending_indicator_is_for_connected_account(indicator, account_address.as_deref())
                    && !self.hud_placements.contains_indicator(*pending_id)
            })
    }

    /// Authoritative fills consume in-flight market-order projections so the
    /// optimistic position delta does not double-count a fill the websocket
    /// has already delivered (the REST ack that clears the indicator can lag
    /// it by seconds). Matching by symbol + side + time is heuristic, but it
    /// only ever shrinks projections toward the authoritative state, never
    /// inflates them.
    pub(crate) fn consume_pending_market_order_fills(&mut self, fills: &[UserFill]) -> bool {
        let Some(account_address) = self.connected_order_account_address() else {
            return false;
        };
        let mut changed = false;
        for fill in fills {
            let Some(mut remaining) = parse_positive_finite_number(&fill.sz) else {
                continue;
            };
            let fill_is_buy = fill.side == "B";
            let matching_ids: Vec<u64> = self
                .pending_order_indicators
                .iter()
                .filter(|(_, indicator)| {
                    indicator.kind == PendingOrderIndicatorKind::MarketPlacing
                        && pending_indicator_is_for_account(indicator, &account_address)
                        && indicator.symbol == fill.coin
                        && indicator.is_buy == fill_is_buy
                        && fill_time_covers_indicator(fill.time, indicator.created_at_ms)
                })
                .map(|(pending_id, _)| *pending_id)
                .collect();
            for pending_id in matching_ids {
                if remaining <= 0.0 {
                    break;
                }
                let Some(indicator) = self.pending_order_indicators.get_mut(&pending_id) else {
                    continue;
                };
                let Some(size) = parse_positive_finite_number(&indicator.size) else {
                    self.pending_order_indicators.remove(&pending_id);
                    changed = true;
                    continue;
                };
                if size <= remaining * (1.0 + 1e-9) {
                    remaining -= size;
                    self.pending_order_indicators.remove(&pending_id);
                } else {
                    indicator.size = (size - remaining).to_string();
                    remaining = 0.0;
                }
                changed = true;
            }
        }
        if changed {
            self.sync_all_chart_orders();
        }
        changed
    }

    pub(crate) fn pending_cancel_indicator_order(
        &self,
        pending_id: Option<u64>,
    ) -> Option<(u64, String)> {
        let indicator = self.pending_order_indicators.get(&pending_id?)?;
        if indicator.kind != PendingOrderIndicatorKind::Cancelling {
            return None;
        }
        Some((indicator.oid?, indicator.symbol.clone()))
    }

    pub(crate) fn pending_modification_price(&self, pending_id: Option<u64>) -> Option<String> {
        let indicator = self.pending_order_indicators.get(&pending_id?)?;
        (indicator.kind == PendingOrderIndicatorKind::Modifying).then(|| indicator.price.clone())
    }

    pub(crate) fn clear_pending_order_indicator(&mut self, pending_id: Option<u64>) -> bool {
        let Some(pending_id) = pending_id else {
            return false;
        };
        let changed = self.pending_order_indicators.remove(&pending_id).is_some();
        if changed {
            self.sync_all_chart_orders();
        }
        changed
    }

    pub(crate) fn expire_pending_order_indicators(&mut self) -> bool {
        let now_ms = Self::now_ms();
        let before = self.pending_order_indicators.len();
        self.pending_order_indicators
            .retain(|_, indicator| indicator_is_fresh(indicator.created_at_ms, now_ms));
        let changed = before != self.pending_order_indicators.len();
        if changed {
            self.sync_all_chart_orders();
        }
        changed
    }

    pub(crate) fn pending_order_indicators_for_symbol(
        &self,
        symbol: &str,
    ) -> Vec<(u64, PendingOrderIndicator)> {
        let account_address = self.connected_order_account_address();
        self.pending_order_indicators
            .iter()
            .filter_map(|(pending_id, indicator)| {
                (pending_indicator_is_for_connected_account(indicator, account_address.as_deref())
                    && indicator.symbol == symbol)
                    .then_some((*pending_id, indicator.clone()))
            })
            .collect()
    }

    fn next_pending_order_indicator_id(&self, created_at_ms: u64) -> u64 {
        let mut pending_id = created_at_ms.max(1);
        while self.pending_order_indicators.contains_key(&pending_id) {
            pending_id = pending_id.saturating_add(1);
        }
        pending_id
    }
}

/// Aggregates same-symbol market-order indicators into one projected delta.
/// The estimated price is the size-weighted average of the contributing
/// orders, omitted entirely when the sides disagree (a single "~price" for a
/// netted buy/sell pair would be the wrong order's price).
struct PositionDeltaAccumulator {
    symbol: String,
    signed_size: f64,
    first_is_buy: bool,
    mixed_sides: bool,
    price_volume: f64,
    priced_volume: f64,
}

impl PositionDeltaAccumulator {
    fn new(symbol: String, is_buy: bool) -> Self {
        Self {
            symbol,
            signed_size: 0.0,
            first_is_buy: is_buy,
            mixed_sides: false,
            price_volume: 0.0,
            priced_volume: 0.0,
        }
    }

    fn add(&mut self, size: f64, is_buy: bool, price: &str) {
        self.signed_size += if is_buy { size } else { -size };
        if is_buy != self.first_is_buy {
            self.mixed_sides = true;
        }
        if let Some(price) = parse_positive_finite_number(price) {
            self.price_volume += price * size;
            self.priced_volume += size;
        }
    }

    fn finish(self) -> ProjectedPositionDelta {
        let estimated_price = (!self.mixed_sides && self.priced_volume > 0.0)
            .then(|| self.price_volume / self.priced_volume);
        ProjectedPositionDelta {
            symbol: self.symbol,
            signed_size: self.signed_size,
            estimated_price,
        }
    }
}

fn pending_indicator_is_for_connected_account(
    indicator: &PendingOrderIndicator,
    connected_account: Option<&str>,
) -> bool {
    connected_account.is_some_and(|account| pending_indicator_is_for_account(indicator, account))
}

fn pending_indicator_is_for_account(
    indicator: &PendingOrderIndicator,
    account_address: &str,
) -> bool {
    order_account_addresses_match(&indicator.account_address, account_address)
}

/// The exchange commits orders before the place ack returns, so the
/// websocket can deliver the confirmed open order while the Placing
/// indicator is still alive. Mirrors the chart-overlay dedup so the Orders
/// tab never shows the same order twice.
fn placing_indicator_matches_confirmed_order(
    indicator: &PendingOrderIndicator,
    open_orders: &[OpenOrder],
) -> bool {
    let Some(size) = parse_positive_finite_number(&indicator.size) else {
        return false;
    };
    let Some(price) = parse_positive_finite_number(&indicator.price) else {
        return false;
    };
    open_orders.iter().any(|order| {
        order.coin == indicator.symbol
            && open_order_is_buy(&order.side) == Some(indicator.is_buy)
            && order
                .limit_px
                .parse::<f64>()
                .is_ok_and(|px| values_match_approx(px, price))
            && order
                .sz
                .parse::<f64>()
                .is_ok_and(|sz| values_match_approx(sz, size))
    })
}

/// Exchange fill timestamps and local indicator creation use different
/// clocks; tolerate a small skew so a fill that genuinely belongs to an
/// in-flight order is not ignored. Erring open here only delays the
/// consumption until the REST ack clears the indicator.
fn fill_time_covers_indicator(fill_time_ms: u64, created_at_ms: u64) -> bool {
    fill_time_ms.saturating_add(2_000) >= created_at_ms
}

fn open_order_is_buy(side: &str) -> Option<bool> {
    match side {
        "B" => Some(true),
        "A" => Some(false),
        _ => None,
    }
}

fn indicator_is_fresh(created_at_ms: u64, now_ms: u64) -> bool {
    now_ms.saturating_sub(created_at_ms) <= PENDING_ORDER_INDICATOR_TTL_MS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_state::ChartInstance;
    use crate::timeframe::Timeframe;

    const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

    fn terminal_with_chart() -> TradingTerminal {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.charts.clear();
        terminal
            .charts
            .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
        terminal
    }

    fn open_order(oid: u64, side: &str) -> OpenOrder {
        OpenOrder {
            coin: "BTC".to_string(),
            side: side.to_string(),
            limit_px: "100".to_string(),
            sz: "1".to_string(),
            oid,
            timestamp: 1,
            reduce_only: Some(false),
            is_trigger: None,
            order_type: None,
            tif: None,
            trigger_px: None,
        }
    }

    #[test]
    fn pending_order_indicator_debug_redacts_order_values_without_changing_them() {
        let indicator = PendingOrderIndicator {
            account_address: TEST_ACCOUNT.to_string(),
            symbol: "private-pending-symbol-sentinel".to_string(),
            oid: Some(98_765_432),
            is_buy: true,
            size: "12345.6789".to_string(),
            price: "98765.4321".to_string(),
            kind: PendingOrderIndicatorKind::Placing,
            created_at_ms: 7,
        };

        let rendered = format!("{indicator:?}");

        assert!(rendered.contains("PendingOrderIndicator"));
        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains(TEST_ACCOUNT));
        for value in [
            "private-pending-symbol-sentinel",
            "98765432",
            "12345.6789",
            "98765.4321",
        ] {
            assert!(!rendered.contains(value), "{rendered}");
        }
        assert_eq!(indicator.account_address, TEST_ACCOUNT);
        assert_eq!(indicator.symbol, "private-pending-symbol-sentinel");
        assert_eq!(indicator.oid, Some(98_765_432));
        assert_eq!(indicator.size, "12345.6789");
        assert_eq!(indicator.price, "98765.4321");
    }

    #[test]
    fn optimistic_order_helpers_redact_financial_values_without_changing_them() {
        let row = OptimisticOrderRowState::Modifying {
            price: "87654.3219".to_string(),
        };
        let delta = ProjectedPositionDelta {
            symbol: "private-projection-symbol-sentinel".to_string(),
            signed_size: -12_345.678_9,
            estimated_price: Some(98_765.432_1),
        };

        let rendered = format!("{row:?} {delta:?}");

        assert!(rendered.contains("<redacted>"), "{rendered}");
        for value in [
            "87654.3219".to_string(),
            "private-projection-symbol-sentinel".to_string(),
            format!("{:?}", delta.signed_size),
            format!("{:?}", delta.estimated_price.unwrap_or_default()),
        ] {
            assert!(!rendered.contains(&value), "{rendered}");
        }
        assert_eq!(
            row,
            OptimisticOrderRowState::Modifying {
                price: "87654.3219".to_string()
            }
        );
        assert_eq!(delta.symbol, "private-projection-symbol-sentinel");
        assert_eq!(delta.signed_size.to_bits(), (-12_345.678_9_f64).to_bits());
        assert_eq!(
            delta.estimated_price.map(f64::to_bits),
            Some(98_765.432_1_f64.to_bits())
        );
    }

    #[test]
    fn pending_market_order_uses_loading_pulse_instead_of_order_line() {
        let mut terminal = terminal_with_chart();

        let pending_id = terminal.add_pending_market_order_placement_indicator(
            terminal.connected_address.clone().unwrap_or_default(),
            "BTC".to_string(),
            true,
            "1".to_string(),
            "100".to_string(),
        );

        assert!(pending_id.is_some());
        let chart = &terminal.charts.get(&1).unwrap().chart;
        assert!(chart.active_orders.is_empty());
        assert!(chart.hud_order_animation_active());
    }

    #[test]
    fn clearing_pending_market_order_removes_loading_pulse() {
        let mut terminal = terminal_with_chart();
        let pending_id = terminal.add_pending_market_order_placement_indicator(
            terminal.connected_address.clone().unwrap_or_default(),
            "BTC".to_string(),
            false,
            "1".to_string(),
            "100".to_string(),
        );

        assert!(terminal.clear_pending_order_indicator(pending_id));

        let chart = &terminal.charts.get(&1).unwrap().chart;
        assert!(chart.active_orders.is_empty());
        assert!(!chart.hud_order_animation_active());
    }

    #[test]
    fn pending_limit_order_still_uses_order_line() {
        let mut terminal = terminal_with_chart();

        let pending_id = terminal.add_pending_order_placement_indicator(
            terminal.connected_address.clone().unwrap_or_default(),
            "BTC".to_string(),
            true,
            "1".to_string(),
            "100".to_string(),
        );

        assert!(pending_id.is_some());
        let chart = &terminal.charts.get(&1).unwrap().chart;
        assert_eq!(chart.active_orders.len(), 1);
        assert!(!chart.hud_order_animation_active());
    }

    #[test]
    fn invalid_size_or_price_creates_no_indicator() {
        let mut terminal = terminal_with_chart();

        let bad_size = terminal.add_pending_order_placement_indicator(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            true,
            "abc".to_string(),
            "100".to_string(),
        );
        let bad_price = terminal.add_pending_order_placement_indicator(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            true,
            "1".to_string(),
            "-5".to_string(),
        );

        assert_eq!(bad_size, None);
        assert_eq!(bad_price, None);
        assert!(terminal.pending_order_indicators.is_empty());
    }

    #[test]
    fn cancellation_indicator_rejects_unknown_side() {
        let mut terminal = terminal_with_chart();

        let pending_id = terminal.add_pending_order_cancellation_indicator(
            TEST_ACCOUNT.to_string(),
            &open_order(42, "X"),
        );

        assert_eq!(pending_id, None);
        assert!(terminal.pending_order_indicators.is_empty());
    }

    #[test]
    fn has_pending_cancel_indicator_matches_kind_and_oid() {
        let mut terminal = terminal_with_chart();

        let placement_id = terminal.add_pending_order_placement_indicator(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            true,
            "1".to_string(),
            "100".to_string(),
        );
        assert!(placement_id.is_some());
        assert!(!terminal.has_pending_cancel_indicator(42));

        let cancel_id = terminal.add_pending_order_cancellation_indicator(
            TEST_ACCOUNT.to_string(),
            &open_order(42, "B"),
        );
        assert!(cancel_id.is_some());
        assert!(terminal.has_pending_cancel_indicator(42));
        assert!(!terminal.has_pending_cancel_indicator(43));
        assert_eq!(
            terminal.pending_cancel_indicator_order(cancel_id),
            Some((42, "BTC".to_string()))
        );
        assert_eq!(terminal.pending_cancel_indicator_order(placement_id), None);
    }

    #[test]
    fn modification_price_lookup_only_matches_modifying_indicators() {
        let mut terminal = terminal_with_chart();

        let modify_id = terminal.add_pending_order_modification_indicator(
            TEST_ACCOUNT.to_string(),
            &open_order(42, "B"),
            "111".to_string(),
        );
        let cancel_id = terminal.add_pending_order_cancellation_indicator(
            TEST_ACCOUNT.to_string(),
            &open_order(43, "B"),
        );

        assert_eq!(
            terminal.pending_modification_price(modify_id),
            Some("111".to_string())
        );
        assert_eq!(terminal.pending_modification_price(cancel_id), None);
        assert_eq!(terminal.pending_modification_price(None), None);
    }

    #[test]
    fn indicators_expire_after_ttl_and_resync_charts() {
        let mut terminal = terminal_with_chart();
        let pending_id = terminal.add_pending_order_placement_indicator(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            true,
            "1".to_string(),
            "100".to_string(),
        );
        let pending_id = pending_id.expect("indicator should be created");
        assert_eq!(
            terminal.charts.get(&1).unwrap().chart.active_orders.len(),
            1
        );

        // A fresh indicator survives an expiry pass untouched.
        assert!(!terminal.expire_pending_order_indicators());
        assert_eq!(terminal.pending_order_indicators.len(), 1);

        if let Some(indicator) = terminal.pending_order_indicators.get_mut(&pending_id) {
            indicator.created_at_ms = indicator
                .created_at_ms
                .saturating_sub(PENDING_ORDER_INDICATOR_TTL_MS + 1);
        }

        assert!(terminal.expire_pending_order_indicators());
        assert!(terminal.pending_order_indicators.is_empty());
        assert!(
            terminal
                .charts
                .get(&1)
                .unwrap()
                .chart
                .active_orders
                .is_empty()
        );
    }

    #[test]
    fn indicators_for_other_accounts_are_not_returned_or_drawn() {
        let mut terminal = terminal_with_chart();

        let pending_id = terminal.add_pending_order_placement_indicator(
            "0xdef0000000000000000000000000000000000000".to_string(),
            "BTC".to_string(),
            true,
            "1".to_string(),
            "100".to_string(),
        );

        assert!(pending_id.is_some());
        assert!(
            terminal
                .pending_order_indicators_for_symbol("BTC")
                .is_empty()
        );
        assert!(
            terminal
                .charts
                .get(&1)
                .unwrap()
                .chart
                .active_orders
                .is_empty()
        );
    }

    fn add_market_indicator(
        terminal: &mut TradingTerminal,
        symbol: &str,
        is_buy: bool,
        size: &str,
    ) {
        let pending_id = terminal.add_pending_market_order_placement_indicator(
            TEST_ACCOUNT.to_string(),
            symbol.to_string(),
            is_buy,
            size.to_string(),
            "100".to_string(),
        );
        assert!(pending_id.is_some());
    }

    #[test]
    fn optimistic_projections_are_empty_when_setting_disabled() {
        let mut terminal = terminal_with_chart();
        terminal.optimistic_account_updates = false;
        add_market_indicator(&mut terminal, "BTC", true, "1");
        terminal.add_pending_order_placement_indicator(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            true,
            "1".to_string(),
            "100".to_string(),
        );
        let cancel_id = terminal.add_pending_order_cancellation_indicator(
            TEST_ACCOUNT.to_string(),
            &open_order(42, "B"),
        );
        assert!(cancel_id.is_some());

        assert!(terminal.optimistic_position_deltas().is_empty());
        assert!(terminal.optimistic_open_order_rows().is_empty());
        assert_eq!(terminal.optimistic_open_order_row_state(42), None);
    }

    #[test]
    fn optimistic_position_deltas_aggregate_market_orders_per_symbol() {
        let mut terminal = terminal_with_chart();
        terminal.optimistic_account_updates = true;
        add_market_indicator(&mut terminal, "BTC", true, "2");
        add_market_indicator(&mut terminal, "BTC", false, "0.5");
        add_market_indicator(&mut terminal, "ETH", false, "3");
        // Limit placements and other accounts' orders must not project.
        terminal.add_pending_order_placement_indicator(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            true,
            "10".to_string(),
            "100".to_string(),
        );
        terminal.add_pending_market_order_placement_indicator(
            "0xdef0000000000000000000000000000000000000".to_string(),
            "BTC".to_string(),
            true,
            "10".to_string(),
            "100".to_string(),
        );

        let deltas = terminal.optimistic_position_deltas();
        assert_eq!(deltas.len(), 2);
        let btc = deltas.iter().find(|d| d.symbol == "BTC").unwrap();
        assert!((btc.signed_size - 1.5).abs() < 1e-12);
        let eth = deltas.iter().find(|d| d.symbol == "ETH").unwrap();
        assert!((eth.signed_size + 3.0).abs() < 1e-12);
        assert_eq!(
            terminal.optimistic_position_delta_for_symbol("BTC"),
            Some(btc.signed_size)
        );
        assert_eq!(terminal.optimistic_position_delta_for_symbol("SOL"), None);
    }

    #[test]
    fn optimistic_open_order_rows_include_only_own_limit_placements() {
        let mut terminal = terminal_with_chart();
        terminal.optimistic_account_updates = true;
        terminal.add_pending_order_placement_indicator(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            true,
            "1".to_string(),
            "100".to_string(),
        );
        add_market_indicator(&mut terminal, "BTC", true, "1");
        terminal.add_pending_order_placement_indicator(
            "0xdef0000000000000000000000000000000000000".to_string(),
            "ETH".to_string(),
            true,
            "1".to_string(),
            "100".to_string(),
        );

        let rows = terminal.optimistic_open_order_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "BTC");
        assert_eq!(rows[0].kind, PendingOrderIndicatorKind::Placing);
    }

    #[test]
    fn optimistic_row_state_reports_cancelling_and_modifying() {
        let mut terminal = terminal_with_chart();
        terminal.optimistic_account_updates = true;
        terminal.add_pending_order_cancellation_indicator(
            TEST_ACCOUNT.to_string(),
            &open_order(42, "B"),
        );
        terminal.add_pending_order_modification_indicator(
            TEST_ACCOUNT.to_string(),
            &open_order(43, "B"),
            "111".to_string(),
        );

        assert_eq!(
            terminal.optimistic_open_order_row_state(42),
            Some(OptimisticOrderRowState::Cancelling)
        );
        assert_eq!(
            terminal.optimistic_open_order_row_state(43),
            Some(OptimisticOrderRowState::Modifying {
                price: "111".to_string()
            })
        );
        assert_eq!(terminal.optimistic_open_order_row_state(44), None);
    }

    fn account_data_with_open_orders(orders: Vec<OpenOrder>) -> crate::account::AccountData {
        crate::account::AccountData {
            fetch_scope: Default::default(),
            request_weight_estimate: 0,
            account_abstraction: Default::default(),
            clearinghouse: crate::account::ClearinghouseState {
                margin_summary: crate::account::MarginSummary {
                    account_value: "0".to_string(),
                    total_ntl_pos: "0".to_string(),
                    total_margin_used: "0".to_string(),
                },
                cross_margin_summary: None,
                cross_maintenance_margin_used: None,
                withdrawable: "0".to_string(),
                asset_positions: Vec::new(),
            },
            clearinghouses_by_dex: std::collections::HashMap::new(),
            spot: crate::account::SpotClearinghouseState {
                balances: Vec::new(),
                portfolio_margin_enabled: false,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            open_orders: orders,
            fills: Vec::new(),
            funding_history: Vec::new(),
            fee_rates: crate::account::UserFeeRates::default(),
            completeness: crate::account::AccountDataCompleteness::default(),
            fetched_at_ms: 1,
        }
    }

    fn user_fill(coin: &str, side: &str, sz: &str, time: u64) -> UserFill {
        UserFill {
            coin: coin.to_string(),
            px: "100".to_string(),
            sz: sz.to_string(),
            side: side.to_string(),
            time,
            hash: None,
            tid: None,
            oid: None,
            dir: "Open Long".to_string(),
            closed_pnl: "0".to_string(),
            fee: "0".to_string(),
            fee_token: None,
        }
    }

    fn zero_size_trigger_order(oid: u64) -> OpenOrder {
        OpenOrder {
            coin: "BTC".to_string(),
            side: "A".to_string(),
            limit_px: "0".to_string(),
            sz: "0.0".to_string(),
            oid,
            timestamp: 1,
            reduce_only: Some(true),
            is_trigger: Some(true),
            order_type: Some("Stop Market".to_string()),
            tif: None,
            trigger_px: Some("90".to_string()),
        }
    }

    #[test]
    fn cancel_indicator_created_for_zero_size_trigger_order() {
        let mut terminal = terminal_with_chart();

        let pending_id = terminal.add_pending_order_cancellation_indicator(
            TEST_ACCOUNT.to_string(),
            &zero_size_trigger_order(42),
        );

        assert!(pending_id.is_some());
        assert!(terminal.has_pending_cancel_indicator(42));
        assert_eq!(
            terminal.pending_cancel_indicator_order(pending_id),
            Some((42, "BTC".to_string()))
        );
    }

    #[test]
    fn modification_indicator_allows_zero_size_but_requires_valid_price() {
        let mut terminal = terminal_with_chart();

        let valid_price = terminal.add_pending_order_modification_indicator(
            TEST_ACCOUNT.to_string(),
            &zero_size_trigger_order(42),
            "111".to_string(),
        );
        let invalid_price = terminal.add_pending_order_modification_indicator(
            TEST_ACCOUNT.to_string(),
            &zero_size_trigger_order(43),
            "abc".to_string(),
        );

        assert!(valid_price.is_some());
        assert_eq!(invalid_price, None);
        assert_eq!(
            terminal.pending_modification_price(valid_price),
            Some("111".to_string())
        );
    }

    #[test]
    fn has_pending_cancel_indicator_is_scoped_to_the_connected_account() {
        let mut terminal = terminal_with_chart();
        let cancel_id = terminal.add_pending_order_cancellation_indicator(
            "0xdef0000000000000000000000000000000000000".to_string(),
            &open_order(42, "B"),
        );
        assert!(cancel_id.is_some());

        assert!(!terminal.has_pending_cancel_indicator(42));
    }

    #[test]
    fn pending_indicators_use_trimmed_connected_account() {
        let mut terminal = terminal_with_chart();
        terminal.connected_address = Some(format!(" {TEST_ACCOUNT} "));
        terminal.optimistic_account_updates = true;

        add_market_indicator(&mut terminal, "BTC", true, "1");
        let cancel_id = terminal.add_pending_order_cancellation_indicator(
            TEST_ACCOUNT.to_string(),
            &open_order(42, "B"),
        );
        assert!(cancel_id.is_some());

        assert!(terminal.has_pending_cancel_indicator(42));
        assert_eq!(terminal.pending_order_indicators_for_symbol("BTC").len(), 2);
        assert_eq!(terminal.optimistic_position_deltas().len(), 1);
    }

    #[test]
    fn pending_indicators_match_account_case_insensitively() {
        let mut terminal = terminal_with_chart();
        terminal.connected_address = Some(TEST_ACCOUNT.to_ascii_uppercase());
        terminal.optimistic_account_updates = true;

        add_market_indicator(&mut terminal, "BTC", true, "1");
        let cancel_id = terminal.add_pending_order_cancellation_indicator(
            TEST_ACCOUNT.to_string(),
            &open_order(42, "B"),
        );
        assert!(cancel_id.is_some());

        assert!(terminal.has_pending_cancel_indicator(42));
        assert_eq!(terminal.pending_order_indicators_for_symbol("BTC").len(), 2);
        assert_eq!(terminal.optimistic_position_deltas().len(), 1);
    }

    #[test]
    fn pending_indicators_ignore_blank_connected_account() {
        let mut terminal = terminal_with_chart();
        terminal.connected_address = Some("   ".to_string());
        terminal.optimistic_account_updates = true;

        add_market_indicator(&mut terminal, "BTC", true, "1");
        let cancel_id = terminal.add_pending_order_cancellation_indicator(
            TEST_ACCOUNT.to_string(),
            &open_order(42, "B"),
        );
        assert!(cancel_id.is_some());
        let created_at_ms = terminal
            .pending_order_indicators
            .values()
            .find(|indicator| indicator.kind == PendingOrderIndicatorKind::MarketPlacing)
            .expect("market indicator")
            .created_at_ms;

        assert!(!terminal.has_pending_cancel_indicator(42));
        assert!(
            terminal
                .pending_order_indicators_for_symbol("BTC")
                .is_empty()
        );
        assert!(terminal.optimistic_position_deltas().is_empty());
        assert!(terminal.optimistic_open_order_rows().is_empty());
        assert_eq!(terminal.optimistic_open_order_row_state(42), None);
        assert!(!terminal.consume_pending_market_order_fills(&[user_fill(
            "BTC",
            "B",
            "1",
            created_at_ms + 50,
        )]));
    }

    #[test]
    fn pending_indicator_account_predicate_requires_matching_connected_account() {
        let mut terminal = terminal_with_chart();
        let pending_id = terminal
            .add_pending_order_placement_indicator(
                TEST_ACCOUNT.to_string(),
                "BTC".to_string(),
                true,
                "1".to_string(),
                "100".to_string(),
            )
            .expect("indicator should be created");
        let indicator = terminal
            .pending_order_indicators
            .get(&pending_id)
            .expect("indicator should be stored");

        assert!(pending_indicator_is_for_connected_account(
            indicator,
            Some(TEST_ACCOUNT)
        ));
        assert!(pending_indicator_is_for_connected_account(
            indicator,
            Some(&format!(" {} ", TEST_ACCOUNT.to_ascii_uppercase()))
        ));
        assert!(!pending_indicator_is_for_connected_account(indicator, None));
        assert!(!pending_indicator_is_for_connected_account(
            indicator,
            Some("0xdef0000000000000000000000000000000000000")
        ));
    }

    #[test]
    fn placing_rows_are_deduplicated_against_confirmed_open_orders() {
        let mut terminal = terminal_with_chart();
        terminal.optimistic_account_updates = true;
        // The confirmed snapshot formats values differently ("100.0" vs
        // "100"); the dedup must match numerically, not textually.
        let mut confirmed = open_order(42, "B");
        confirmed.limit_px = "100.0".to_string();
        confirmed.sz = "1.0".to_string();
        terminal.account_data_address = Some(TEST_ACCOUNT.to_string());
        terminal.account_data = Some(account_data_with_open_orders(vec![confirmed]));

        terminal.add_pending_order_placement_indicator(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            true,
            "1".to_string(),
            "100".to_string(),
        );
        terminal.add_pending_order_placement_indicator(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            true,
            "2".to_string(),
            "95".to_string(),
        );

        let rows = terminal.optimistic_open_order_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].price, "95");
    }

    #[test]
    fn row_state_prefers_cancelling_over_modifying_regardless_of_order() {
        let mut terminal = terminal_with_chart();
        terminal.optimistic_account_updates = true;
        terminal.add_pending_order_modification_indicator(
            TEST_ACCOUNT.to_string(),
            &open_order(42, "B"),
            "111".to_string(),
        );
        terminal.add_pending_order_cancellation_indicator(
            TEST_ACCOUNT.to_string(),
            &open_order(42, "B"),
        );

        assert_eq!(
            terminal.optimistic_open_order_row_state(42),
            Some(OptimisticOrderRowState::Cancelling)
        );
    }

    #[test]
    fn position_deltas_exclude_spot_symbols() {
        let mut terminal = terminal_with_chart();
        terminal.optimistic_account_updates = true;
        terminal.exchange_symbols = vec![crate::api::ExchangeSymbol {
            key: "PURR/USDC".to_string(),
            ticker: "PURR".to_string(),
            category: "spot".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 10_000,
            collateral_token: None,
            sz_decimals: 0,
            max_leverage: 1,
            only_isolated: false,
            market_type: crate::api::MarketType::Spot,
            outcome: None,
        }];
        add_market_indicator(&mut terminal, "PURR/USDC", true, "100");
        add_market_indicator(&mut terminal, "BTC", true, "1");

        let deltas = terminal.optimistic_position_deltas();
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].symbol, "BTC");
    }

    #[test]
    fn estimated_price_is_size_weighted_and_omitted_for_mixed_sides() {
        let mut terminal = terminal_with_chart();
        terminal.optimistic_account_updates = true;
        let add = |terminal: &mut TradingTerminal, is_buy: bool, size: &str, price: &str| {
            let pending_id = terminal.add_pending_market_order_placement_indicator(
                TEST_ACCOUNT.to_string(),
                "BTC".to_string(),
                is_buy,
                size.to_string(),
                price.to_string(),
            );
            assert!(pending_id.is_some());
        };
        add(&mut terminal, true, "1", "100");
        add(&mut terminal, true, "3", "200");

        let deltas = terminal.optimistic_position_deltas();
        assert_eq!(deltas.len(), 1);
        let estimated = deltas[0].estimated_price.expect("weighted price");
        assert!((estimated - 175.0).abs() < 1e-9);

        // A netted buy/sell pair has no single meaningful action price.
        add(&mut terminal, false, "0.5", "150");
        let deltas = terminal.optimistic_position_deltas();
        assert_eq!(deltas[0].estimated_price, None);
    }

    #[test]
    fn ws_fill_consumes_matching_market_indicator() {
        let mut terminal = terminal_with_chart();
        add_market_indicator(&mut terminal, "BTC", true, "1");
        let created_at_ms = terminal
            .pending_order_indicators
            .values()
            .next()
            .expect("indicator")
            .created_at_ms;

        let changed = terminal.consume_pending_market_order_fills(&[user_fill(
            "BTC",
            "B",
            "1",
            created_at_ms + 50,
        )]);

        assert!(changed);
        assert!(terminal.pending_order_indicators.is_empty());
        let chart = &terminal.charts.get(&1).unwrap().chart;
        assert!(!chart.hud_order_animation_active());
    }

    #[test]
    fn ws_partial_fill_shrinks_market_indicator() {
        let mut terminal = terminal_with_chart();
        terminal.optimistic_account_updates = true;
        add_market_indicator(&mut terminal, "BTC", true, "2");
        let created_at_ms = terminal
            .pending_order_indicators
            .values()
            .next()
            .expect("indicator")
            .created_at_ms;

        let changed = terminal.consume_pending_market_order_fills(&[user_fill(
            "BTC",
            "B",
            "0.5",
            created_at_ms + 50,
        )]);

        assert!(changed);
        let remaining = terminal
            .optimistic_position_delta_for_symbol("BTC")
            .expect("delta");
        assert!((remaining - 1.5).abs() < 1e-9);
    }

    #[test]
    fn ws_fill_ignores_mismatched_side_symbol_and_stale_fills() {
        let mut terminal = terminal_with_chart();
        add_market_indicator(&mut terminal, "BTC", true, "1");
        let created_at_ms = terminal
            .pending_order_indicators
            .values()
            .next()
            .expect("indicator")
            .created_at_ms;

        let changed = terminal.consume_pending_market_order_fills(&[
            user_fill("BTC", "A", "1", created_at_ms + 50),
            user_fill("ETH", "B", "1", created_at_ms + 50),
            user_fill("BTC", "B", "1", created_at_ms.saturating_sub(10_000)),
        ]);

        assert!(!changed);
        assert_eq!(terminal.pending_order_indicators.len(), 1);
    }

    #[test]
    fn same_timestamp_indicator_ids_do_not_collide() {
        let mut terminal = terminal_with_chart();

        let first_id = terminal
            .add_pending_order_placement_indicator(
                TEST_ACCOUNT.to_string(),
                "BTC".to_string(),
                true,
                "1".to_string(),
                "100".to_string(),
            )
            .expect("indicator should be created");
        let created_at_ms = terminal
            .pending_order_indicators
            .get(&first_id)
            .expect("indicator should be stored")
            .created_at_ms;

        let second_id = terminal.next_pending_order_indicator_id(created_at_ms);

        assert_ne!(first_id, second_id);
    }
}

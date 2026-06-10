use crate::account::OpenOrder;
use crate::app_state::TradingTerminal;
use crate::helpers::parse_positive_finite_number;

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

#[derive(Debug, Clone)]
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

struct PendingOrderIndicatorInput {
    account_address: String,
    symbol: String,
    oid: Option<u64>,
    is_buy: bool,
    size: String,
    price: String,
    kind: PendingOrderIndicatorKind,
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
        parse_positive_finite_number(&input.size)?;
        parse_positive_finite_number(&input.price)?;

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

    pub(crate) fn has_pending_cancel_indicator(&self, oid: u64) -> bool {
        self.pending_order_indicators.values().any(|indicator| {
            indicator.kind == PendingOrderIndicatorKind::Cancelling && indicator.oid == Some(oid)
        })
    }

    pub(crate) fn pending_cancel_indicator_oid(&self, pending_id: Option<u64>) -> Option<u64> {
        let indicator = self.pending_order_indicators.get(&pending_id?)?;
        (indicator.kind == PendingOrderIndicatorKind::Cancelling)
            .then_some(indicator.oid)
            .flatten()
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
        let account_address = self.connected_address.as_deref();
        self.pending_order_indicators
            .iter()
            .filter_map(|(pending_id, indicator)| {
                (account_address == Some(indicator.account_address.as_str())
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
        assert_eq!(terminal.pending_cancel_indicator_oid(cancel_id), Some(42));
        assert_eq!(terminal.pending_cancel_indicator_oid(placement_id), None);
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

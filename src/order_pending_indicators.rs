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

impl TradingTerminal {
    pub(crate) fn add_pending_order_placement_indicator(
        &mut self,
        account_address: String,
        symbol: String,
        is_buy: bool,
        size: String,
        price: String,
    ) -> Option<u64> {
        parse_positive_finite_number(&size)?;
        parse_positive_finite_number(&price)?;

        let created_at_ms = Self::now_ms();
        let pending_id = self.next_pending_order_indicator_id(created_at_ms);
        self.pending_order_indicators.insert(
            pending_id,
            PendingOrderIndicator {
                account_address,
                symbol,
                oid: None,
                is_buy,
                size,
                price,
                kind: PendingOrderIndicatorKind::Placing,
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
        parse_positive_finite_number(&order.sz)?;
        parse_positive_finite_number(&order.limit_px)?;

        let created_at_ms = Self::now_ms();
        let pending_id = self.next_pending_order_indicator_id(created_at_ms);
        self.pending_order_indicators.insert(
            pending_id,
            PendingOrderIndicator {
                account_address,
                symbol: order.coin.clone(),
                oid: Some(order.oid),
                is_buy,
                size: order.sz.clone(),
                price: order.limit_px.clone(),
                kind: PendingOrderIndicatorKind::Cancelling,
                created_at_ms,
            },
        );
        self.sync_all_chart_orders();
        Some(pending_id)
    }

    pub(crate) fn add_pending_order_modification_indicator(
        &mut self,
        account_address: String,
        order: &OpenOrder,
        new_price: String,
    ) -> Option<u64> {
        let is_buy = open_order_is_buy(&order.side)?;
        parse_positive_finite_number(&order.sz)?;
        parse_positive_finite_number(&new_price)?;

        let created_at_ms = Self::now_ms();
        let pending_id = self.next_pending_order_indicator_id(created_at_ms);
        self.pending_order_indicators.insert(
            pending_id,
            PendingOrderIndicator {
                account_address,
                symbol: order.coin.clone(),
                oid: Some(order.oid),
                is_buy,
                size: order.sz.clone(),
                price: new_price,
                kind: PendingOrderIndicatorKind::Modifying,
                created_at_ms,
            },
        );
        self.sync_all_chart_orders();
        Some(pending_id)
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

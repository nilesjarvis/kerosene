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

    fn terminal_with_chart() -> TradingTerminal {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
        terminal.charts.clear();
        terminal
            .charts
            .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
        terminal
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
}

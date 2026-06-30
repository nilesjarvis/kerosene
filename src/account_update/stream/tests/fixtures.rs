use crate::account::{
    AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary, OpenOrder,
    SpotClearinghouseState, UserFill,
};
use crate::app_state::TradingTerminal;
use crate::signing::{ChaseLifecycle, ChaseOrder, ChaseVerificationReason};

use std::time::Instant;

pub(crate) fn open_order(oid: u64, reduce_only: Option<bool>) -> OpenOrder {
    OpenOrder {
        coin: "BTC".to_string(),
        side: "B".to_string(),
        limit_px: "100".to_string(),
        sz: "0.1".to_string(),
        oid,
        timestamp: 1,
        reduce_only,
        is_trigger: None,
        order_type: None,
        tif: None,
        trigger_px: None,
    }
}

pub(crate) fn fill(time: u64) -> UserFill {
    UserFill {
        coin: "BTC".to_string(),
        px: "100".to_string(),
        sz: "0.1".to_string(),
        side: "B".to_string(),
        time,
        hash: None,
        tid: None,
        oid: None,
        dir: "Open Long".to_string(),
        closed_pnl: "0".to_string(),
        fee: "0.01".to_string(),
        fee_token: None,
    }
}

pub(crate) fn fill_with_oid(time: u64, oid: u64, px: &str, sz: &str) -> UserFill {
    let mut fill = fill(time);
    fill.oid = Some(oid);
    fill.px = px.to_string();
    fill.sz = sz.to_string();
    fill
}

pub(crate) fn chase_order() -> ChaseOrder {
    ChaseOrder {
        id: 1,
        coin: "BTC".to_string(),
        account_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: "agent-key".to_string().into(),
        is_buy: true,
        target_size: 1.0,
        filled_size: 0.0,
        remaining_size: 1.0,
        known_oids: vec![42],
        current_cloid: None,
        place_attempt_count: 0,
        asset: 0,
        sz_decimals: 5,
        is_spot: false,
        reduce_only: false,
        current_oid: Some(42),
        current_price: 100.0,
        current_price_wire: "100".to_string(),
        initial_price: 100.0,
        started_at: Instant::now(),
        started_at_ms: 1_000,
        fill_cutoff_ms_by_oid: Vec::new(),
        reprice_count: 0,
        lifecycle: ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Placement,
        },
        last_reprice_at: None,
        desired_price: None,
        stop_reason: None,
        cancel_retries: 0,
    }
}

pub(crate) fn chase_order_by_id(terminal: &TradingTerminal, id: u64) -> &ChaseOrder {
    match terminal.chase_orders.get(&id) {
        Some(chase) => chase,
        None => panic!("chase should remain"),
    }
}

pub(crate) fn account_data_with_timestamp(fetched_at_ms: u64) -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
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
        spot: SpotClearinghouseState {
            balances: Vec::new(),
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: Default::default(),
        completeness: AccountDataCompleteness::default(),
        fetched_at_ms,
    }
}

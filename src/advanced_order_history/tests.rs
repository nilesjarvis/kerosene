use super::{
    ADVANCED_ORDER_HISTORY_LIMIT, AdvancedOrderHistoryEntry, AdvancedOrderHistoryKind,
    prune_advanced_order_history, upsert_advanced_order_history,
};
use crate::twap_state::{
    TwapChildOrder, TwapChildStatus, TwapEvent, TwapEventKind, TwapOrder, TwapOrderInit, TwapStatus,
};

use std::time::{Duration, Instant};

mod chase_snapshot;
mod pruning;
mod twap_snapshot;

fn twap_order(now: Instant) -> TwapOrder {
    TwapOrder::new(TwapOrderInit {
        id: 7,
        coin: "BTC".to_string(),
        display_coin: "BTC".to_string(),
        account_address: "0xabc".to_string(),
        agent_key: "key".to_string().into(),
        is_buy: true,
        target_size: 5.0,
        asset: 0,
        sz_decimals: 3,
        is_spot: false,
        reduce_only: false,
        min_price: 90.0,
        max_price: 110.0,
        randomize: false,
        duration: Duration::from_secs(60),
        slice_count: 4,
        now,
        started_at_ms: 1_000,
    })
}

fn child(
    now: Instant,
    index: u32,
    filled_size: f64,
    avg_price: Option<f64>,
    fee: f64,
) -> TwapChildOrder {
    TwapChildOrder {
        index,
        requested_at: now + Duration::from_millis(index as u64),
        planned_size: 1.0,
        limit_price: 100.0,
        oid: Some(index as u64),
        cloid: None,
        status: TwapChildStatus::Filled,
        exchange_summary: String::new(),
        filled_size,
        avg_price,
        fee,
        retry_count: 0,
    }
}

fn minimal_entry(id: &str) -> AdvancedOrderHistoryEntry {
    AdvancedOrderHistoryEntry {
        id: id.to_string(),
        kind: AdvancedOrderHistoryKind::Twap,
        source_id: 1,
        account_address: "0xabc".to_string(),
        coin: "BTC".to_string(),
        display_coin: "BTC".to_string(),
        is_buy: true,
        target_size: 1.0,
        filled_size: 0.0,
        remaining_size: 1.0,
        average_price: None,
        last_working_price: None,
        gross_notional: 0.0,
        total_fee: 0.0,
        closed_pnl: 0.0,
        min_price: None,
        max_price: None,
        reduce_only: false,
        randomize: false,
        slice_count: 0,
        slices_sent: 0,
        reprice_count: 0,
        status: "Completed".to_string(),
        summary: String::new(),
        started_at_ms: 0,
        completed_at_ms: 1,
        logs: Vec::new(),
        children: Vec::new(),
    }
}

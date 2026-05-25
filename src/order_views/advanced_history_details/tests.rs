use super::formatting::{history_child_id, history_price_range_text, history_runtime_text};
use crate::advanced_order_history::{
    AdvancedOrderHistoryChild, AdvancedOrderHistoryEntry, AdvancedOrderHistoryKind,
};

#[test]
fn history_detail_formatting_handles_ranges_runtime_and_child_ids() {
    let mut entry = minimal_entry();
    assert_eq!(history_runtime_text(&entry), "-");
    assert_eq!(history_price_range_text(&entry), "-");

    entry.started_at_ms = 1_000;
    entry.completed_at_ms = 91_000;
    entry.min_price = Some(100.0);
    entry.max_price = Some(105.0);

    assert_eq!(history_runtime_text(&entry), "1m");
    assert_eq!(history_price_range_text(&entry), "100.00-105.00");

    let mut child = minimal_child();
    assert_eq!(history_child_id(&child), "-");

    child.oid = Some(123);
    assert_eq!(history_child_id(&child), "#123");

    child.cloid = Some("abcdefghijklmno".to_string());
    assert_eq!(history_child_id(&child), "#123 abcdefghij...");

    child.oid = None;
    assert_eq!(history_child_id(&child), "abcdefghij...");
}

fn minimal_entry() -> AdvancedOrderHistoryEntry {
    AdvancedOrderHistoryEntry {
        id: "entry".to_string(),
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
        completed_at_ms: 0,
        logs: Vec::new(),
        children: Vec::new(),
    }
}

fn minimal_child() -> AdvancedOrderHistoryChild {
    AdvancedOrderHistoryChild {
        index: 1,
        elapsed_ms: 0,
        planned_size: 1.0,
        limit_price: 100.0,
        filled_size: 0.0,
        avg_price: None,
        fee: 0.0,
        oid: None,
        cloid: None,
        status: "Pending".to_string(),
        exchange_summary: String::new(),
    }
}

use crate::advanced_order_history::{AdvancedOrderHistoryChild, AdvancedOrderHistoryEntry};
use crate::helpers::{format_duration, format_price, format_timestamp_exact};

use super::super::details::order_child_id_text;

// ---------------------------------------------------------------------------
// Advanced History Detail Formatting
// ---------------------------------------------------------------------------

pub(super) fn history_completed_text(entry: &AdvancedOrderHistoryEntry) -> String {
    if entry.completed_at_ms > 0 {
        format_timestamp_exact(entry.completed_at_ms)
    } else {
        "-".to_string()
    }
}

pub(super) fn history_runtime_text(entry: &AdvancedOrderHistoryEntry) -> String {
    if entry.completed_at_ms > entry.started_at_ms {
        format_duration(entry.completed_at_ms - entry.started_at_ms)
    } else {
        "-".to_string()
    }
}

pub(super) fn history_price_range_text(entry: &AdvancedOrderHistoryEntry) -> String {
    match (entry.min_price, entry.max_price) {
        (Some(min), Some(max)) => format!("{}-{}", format_price(min), format_price(max)),
        _ => "-".to_string(),
    }
}

pub(super) fn history_child_id(child: &AdvancedOrderHistoryChild) -> String {
    order_child_id_text(child.oid, child.cloid.as_deref())
}

use crate::twap_state::{TwapChildOrder, TwapOrder, twap_weighted_average_fill_price};

use super::super::details::{order_child_id_text, short_id};

// ---------------------------------------------------------------------------
// TWAP Detail Formatting
// ---------------------------------------------------------------------------

pub(super) fn twap_pause_text(twap: &TwapOrder) -> String {
    twap.pause_reason
        .map(|reason| reason.label().to_string())
        .unwrap_or_else(|| "-".to_string())
}

pub(super) fn twap_next_retry_text(twap: &TwapOrder) -> String {
    let Some(until) = twap.paused_until else {
        return "-".to_string();
    };
    let seconds = until
        .saturating_duration_since(std::time::Instant::now())
        .as_secs();
    if seconds == 0 {
        "Now".to_string()
    } else {
        format!("{seconds}s")
    }
}

pub(super) fn twap_status_check_text(twap: &TwapOrder) -> String {
    twap.status_check_cloid
        .as_deref()
        .map(|cloid| format!("{} ({})", short_id(cloid), twap.status_check_retries))
        .unwrap_or_else(|| "-".to_string())
}

pub(super) fn child_id_text(child: &TwapChildOrder) -> String {
    order_child_id_text(child.oid, child.cloid.as_deref())
}

pub(super) fn weighted_average_fill_price(twap: &TwapOrder) -> Option<f64> {
    twap_weighted_average_fill_price(twap)
}

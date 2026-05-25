use super::super::summary::{compact_summary, hide_order_oid_references};
use crate::helpers::{format_price, format_relative_time, format_size, positive_finite_value};
use crate::twap_state::{TwapPauseReason, TwapStatus};

// ---------------------------------------------------------------------------
// Advanced Order Row Labels
// ---------------------------------------------------------------------------

pub(super) fn chase_price_label(current_price: f64) -> String {
    positive_finite_value(current_price)
        .map(format_price)
        .unwrap_or_else(|| "Loading".to_string())
}

pub(super) fn chase_meta_label(reprice_count: u32, reduce_only: bool) -> String {
    let reduce_only = if reduce_only { " | RO" } else { "" };
    format!("{reprice_count} reprices{reduce_only}")
}

pub(super) fn chase_size_label(filled_size: f64, target_size: f64, remaining_size: f64) -> String {
    if positive_finite_value(target_size).is_some() {
        format!(
            "{}/{} rem {}",
            format_size(filled_size),
            format_size(target_size),
            format_size(remaining_size)
        )
    } else {
        format_size(remaining_size)
    }
}

pub(super) fn twap_progress_label(filled_size: f64, target_size: f64) -> String {
    format!(
        "{} / {}",
        format_size(filled_size),
        format_size(target_size)
    )
}

pub(super) fn twap_meta_label(
    slices_sent: u32,
    slice_count: u32,
    min_price: f64,
    max_price: f64,
) -> String {
    let range = format!("{}-{}", format_price(min_price), format_price(max_price));
    format!("{slices_sent} of {slice_count} slices | {range}")
}

pub(super) fn twap_status_text(
    status: TwapStatus,
    pause_reason: Option<TwapPauseReason>,
) -> String {
    if let Some(reason) = pause_reason {
        return format!("Paused: {}", reason.label());
    }
    status.label().to_string()
}

pub(super) fn history_progress_label(filled_size: f64, target_size: f64) -> String {
    if positive_finite_value(target_size).is_some() {
        format!(
            "{} / {}",
            format_size(filled_size),
            format_size(target_size)
        )
    } else {
        format_size(filled_size)
    }
}

pub(super) fn history_completed_label(completed_at_ms: u64, now_ms: u64) -> String {
    if completed_at_ms > 0 {
        format!("{} ago", format_relative_time(completed_at_ms, now_ms))
    } else {
        "saved".to_string()
    }
}

pub(super) fn history_summary_label(summary: &str) -> String {
    compact_summary(&hide_order_oid_references(summary))
}

#[cfg(test)]
mod tests;

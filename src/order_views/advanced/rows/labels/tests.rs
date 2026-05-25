use super::*;

#[test]
fn chase_labels_preserve_price_size_and_reduce_only_text() {
    assert_eq!(chase_price_label(100.5), "100.50");
    assert_eq!(chase_price_label(0.0), "Loading");
    assert_eq!(chase_price_label(f64::INFINITY), "Loading");
    assert_eq!(chase_meta_label(3, true), "3 reprices | RO");
    assert_eq!(chase_size_label(0.25, 1.0, 0.75), "0.2500/1.00 rem 0.7500");
    assert_eq!(chase_size_label(0.25, f64::NAN, 0.75), "0.7500");
}

#[test]
fn twap_labels_preserve_progress_meta_and_pause_text() {
    assert_eq!(twap_progress_label(0.5, 2.0), "0.5000 / 2.00");
    assert_eq!(
        twap_meta_label(2, 5, 99.0, 101.0),
        "2 of 5 slices | 99.00-101.00"
    );
    assert_eq!(
        twap_status_text(TwapStatus::Paused, Some(TwapPauseReason::StaleMarketData)),
        "Paused: Stale market data"
    );
    assert_eq!(twap_status_text(TwapStatus::Completed, None), "Done");
}

#[test]
fn history_labels_strip_sensitive_order_ids_and_mark_unsaved_time() {
    assert_eq!(history_completed_label(0, 1_000), "saved");
    assert_eq!(history_progress_label(0.25, f64::INFINITY), "0.2500");
    assert_eq!(
        history_summary_label("Slice unexpectedly rested as oid 123; cancelling"),
        "Slice unexpectedly rested as order; cancelling"
    );
}

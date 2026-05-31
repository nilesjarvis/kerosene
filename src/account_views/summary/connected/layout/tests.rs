use super::compaction::ConnectedSummaryCompaction;

// ---------------------------------------------------------------------------
// Connected Summary Compaction Tests
// ---------------------------------------------------------------------------

#[test]
fn connected_summary_compaction_keeps_full_actions_at_breakpoint() {
    let compaction = ConnectedSummaryCompaction::for_width(1_180.0);

    assert!(!compaction.hide_display_denomination());
    assert!(!compaction.hide_margin_ratio());
    assert!(!compaction.hide_margin_used());
}

#[test]
fn connected_summary_compaction_hides_actions_by_priority() {
    let compaction = ConnectedSummaryCompaction::for_width(1_179.0);
    assert!(compaction.hide_display_denomination());
    assert!(!compaction.hide_margin_ratio());
}

#[test]
fn connected_summary_compaction_hides_metrics_at_small_widths() {
    let compaction = ConnectedSummaryCompaction::for_width(839.0);
    assert!(compaction.hide_margin_ratio());
    assert!(!compaction.hide_margin_used());

    let compaction = ConnectedSummaryCompaction::for_width(719.0);
    assert!(compaction.hide_margin_ratio());
    assert!(compaction.hide_margin_used());
}

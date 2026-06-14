use super::compaction::ConnectedSummaryCompaction;
use super::{SKELETON_METRICS, SKELETON_PULSE_AMP, SKELETON_PULSE_BASE, skeleton_value_alpha};

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

// ---------------------------------------------------------------------------
// Connected Summary Skeleton Tests
// ---------------------------------------------------------------------------

#[test]
fn skeleton_value_alpha_stays_within_calm_band() {
    // The placeholder pulse must never spike harshly: alpha stays inside the
    // [base, base + amp] band (well under 1.0) for every metric at any phase.
    for index in 0..SKELETON_METRICS.len() {
        for step in 0..64 {
            let phase = step as f32 * 0.35; // matches the SpinnerTick increment
            let alpha = skeleton_value_alpha(phase, index);
            assert!(
                alpha >= SKELETON_PULSE_BASE - 1e-4,
                "alpha {alpha} below base at index {index}, step {step}"
            );
            assert!(
                alpha <= SKELETON_PULSE_BASE + SKELETON_PULSE_AMP + 1e-4,
                "alpha {alpha} above band at index {index}, step {step}"
            );
        }
    }
}

#[test]
fn skeleton_metrics_have_positive_finite_widths() {
    for (label_w, value_w) in SKELETON_METRICS {
        assert!(label_w > 0.0 && label_w.is_finite());
        assert!(value_w > 0.0 && value_w.is_finite());
    }
}

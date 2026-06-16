use super::*;

#[test]
fn account_data_completeness_defaults_to_complete_without_warning() {
    let completeness = AccountDataCompleteness::default();

    assert!(completeness.is_complete());
    assert_eq!(completeness.warning_summary(), None);
    assert_eq!(
        completeness.section_warning(AccountDataSection::OpenOrders),
        None
    );
}

#[test]
fn account_data_completeness_marks_sections_as_incomplete_with_context() {
    let mut completeness = AccountDataCompleteness::default();
    completeness.mark_incomplete(
        AccountDataSection::OpenOrders,
        "frontendOpenOrders request failed",
    );
    completeness.mark_incomplete(AccountDataSection::Fills, "userFills parse failed");

    assert!(!completeness.is_complete());
    assert_eq!(
        completeness.section_warning(AccountDataSection::OpenOrders),
        Some("Open orders may be incomplete: frontendOpenOrders request failed".to_string())
    );
    assert_eq!(
        completeness.section_warning(AccountDataSection::Fills),
        Some("Trade history may be incomplete: userFills parse failed".to_string())
    );
    assert_eq!(
        completeness.warning_summary(),
        Some(
            "Partial account data: frontendOpenOrders request failed; userFills parse failed"
                .to_string()
        )
    );
}

#[test]
fn marking_positions_incomplete_clears_actionable() {
    let mut completeness = AccountDataCompleteness::default();
    assert!(completeness.positions_actionable);

    completeness.mark_incomplete(AccountDataSection::Positions, "HIP-3 positions unavailable");

    assert!(!completeness.positions_complete);
    assert!(!completeness.positions_actionable);
}

#[test]
fn degraded_positions_stay_actionable_but_warn() {
    let mut completeness = AccountDataCompleteness::default();

    completeness.mark_degraded(
        AccountDataSection::Positions,
        "Hydromancer API key missing; used Hyperliquid fallback",
    );

    // A usable fallback snapshot: warning surfaces, completeness drops, but the
    // positions stay safe to close/NUKE.
    assert!(!completeness.positions_complete);
    assert!(completeness.positions_actionable);
    assert!(!completeness.is_complete());
    assert_eq!(
        completeness.section_warning(AccountDataSection::Positions),
        Some(
            "Positions may be incomplete: Hydromancer API key missing; used Hyperliquid fallback"
                .to_string()
        )
    );
}

#[test]
fn genuine_incompleteness_outranks_degraded_regardless_of_order() {
    // Safety ratchet: if positions are genuinely missing (e.g. a HIP-3
    // clearinghouse fetch failed and dropped those positions) the snapshot must
    // stay non-actionable even when a later fallback degrade lands on top.
    // NUKE-ing such a snapshot would under-close the omitted positions, so a
    // degrade must never restore actionability.
    let mut degraded_first = AccountDataCompleteness::default();
    degraded_first.mark_degraded(AccountDataSection::Positions, "used Hyperliquid fallback");
    degraded_first.mark_incomplete(AccountDataSection::Positions, "HIP-3 positions unavailable");
    assert!(!degraded_first.positions_actionable);

    let mut incomplete_first = AccountDataCompleteness::default();
    incomplete_first.mark_incomplete(AccountDataSection::Positions, "HIP-3 positions unavailable");
    incomplete_first.mark_degraded(AccountDataSection::Positions, "used Hyperliquid fallback");
    assert!(!incomplete_first.positions_actionable);
}

#[test]
fn degrading_other_sections_leaves_positions_actionable() {
    let mut completeness = AccountDataCompleteness::default();
    completeness.mark_degraded(AccountDataSection::Fills, "used fallback fills");

    assert!(completeness.positions_actionable);
    assert!(completeness.positions_complete);
    assert!(!completeness.fills_complete);
}

#[test]
fn account_data_completeness_deduplicates_warnings() {
    let mut completeness = AccountDataCompleteness::default();
    completeness.mark_incomplete(AccountDataSection::Funding, "userFunding request failed");
    completeness.mark_incomplete(AccountDataSection::Funding, "userFunding request failed");

    assert_eq!(
        completeness.warning_summary(),
        Some("Partial account data: userFunding request failed".to_string())
    );
}

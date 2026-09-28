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
fn spot_balance_completeness_is_independent_from_positions() {
    let mut completeness = AccountDataCompleteness::default();

    completeness.spot_balances_complete = false;

    assert!(!completeness.spot_balances_complete);
    assert!(completeness.positions_complete);
    assert!(completeness.positions_actionable);
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

#[test]
fn account_warnings_preserve_section_membership_and_global_first_occurrence_order() {
    let mut completeness = AccountDataCompleteness::default();
    for (section, warning) in [
        (AccountDataSection::Positions, " shared "),
        (AccountDataSection::OpenOrders, " shared "),
        (AccountDataSection::Fills, "fills unavailable"),
        (AccountDataSection::Positions, "positions fallback"),
        (AccountDataSection::OpenOrders, "orders unavailable"),
        (AccountDataSection::Positions, " shared "),
        (AccountDataSection::OpenOrders, "orders unavailable"),
        (AccountDataSection::Fees, "shared"),
        (AccountDataSection::Funding, "SHARED"),
        (AccountDataSection::Fees, "共有"),
        (AccountDataSection::Fees, "shared"),
    ] {
        completeness.mark_degraded(section, warning);
    }

    assert_eq!(
        completeness.warning_summary().as_deref(),
        Some(
            "Partial account data:  shared ; fills unavailable; positions fallback; \
             orders unavailable; shared; SHARED; 共有"
        )
    );
    for (section, expected) in [
        (
            AccountDataSection::Positions,
            "Positions may be incomplete:  shared ; positions fallback",
        ),
        (
            AccountDataSection::OpenOrders,
            "Open orders may be incomplete:  shared ; orders unavailable",
        ),
        (
            AccountDataSection::Fills,
            "Trade history may be incomplete: fills unavailable",
        ),
        (
            AccountDataSection::Funding,
            "Funding history may be incomplete: SHARED",
        ),
        (
            AccountDataSection::Fees,
            "Fee rates may be incomplete: shared; 共有",
        ),
    ] {
        assert_eq!(
            completeness.section_warning(section).as_deref(),
            Some(expected)
        );
    }
    assert!(completeness.positions_actionable);
}

#[test]
fn empty_warnings_preserve_completeness_changes_and_section_fallback_text() {
    for (section, label) in [
        (AccountDataSection::Positions, "Positions"),
        (AccountDataSection::OpenOrders, "Open orders"),
        (AccountDataSection::Fills, "Trade history"),
        (AccountDataSection::Funding, "Funding history"),
        (AccountDataSection::Fees, "Fee rates"),
    ] {
        let mut completeness = AccountDataCompleteness::default();
        let expected = format!(
            "{label} may be incomplete: refresh account data before relying on this section"
        );

        completeness.mark_degraded(section, "");

        assert!(!completeness.is_complete());
        assert!(completeness.positions_actionable);
        assert_eq!(completeness.warning_summary(), None);
        assert_eq!(
            completeness.section_warning(section).as_deref(),
            Some(expected.as_str())
        );

        completeness.mark_incomplete(section, "");

        assert_eq!(
            completeness.positions_actionable,
            section != AccountDataSection::Positions
        );
        assert_eq!(completeness.warning_summary(), None);
        assert_eq!(
            completeness.section_warning(section).as_deref(),
            Some(expected.as_str())
        );

        // A nonempty whitespace message is retained verbatim, not treated as missing.
        completeness.mark_degraded(section, " \t ");
        assert_eq!(
            completeness.warning_summary().as_deref(),
            Some("Partial account data:  \t ")
        );
        assert_eq!(
            completeness.section_warning(section),
            Some(format!("{label} may be incomplete:  \t "))
        );
    }
}

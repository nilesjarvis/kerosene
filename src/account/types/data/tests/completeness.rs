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
fn account_data_completeness_deduplicates_warnings() {
    let mut completeness = AccountDataCompleteness::default();
    completeness.mark_incomplete(AccountDataSection::Funding, "userFunding request failed");
    completeness.mark_incomplete(AccountDataSection::Funding, "userFunding request failed");

    assert_eq!(
        completeness.warning_summary(),
        Some("Partial account data: userFunding request failed".to_string())
    );
}

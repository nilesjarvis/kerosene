use super::*;

#[test]
fn fee_rate_parse_failure_marks_fees_incomplete() {
    let mut completeness = AccountDataCompleteness::default();
    let rates = fee_rates_from_best_effort_value(
        Err("userFees parse failed: invalid json".to_string()),
        &mut completeness,
    );

    assert_eq!(
        rates.user_cross_rate,
        UserFeeRates::default().user_cross_rate
    );
    assert_eq!(
        completeness.section_warning(AccountDataSection::Fees),
        Some("Fee rates may be incomplete: userFees parse failed: invalid json".to_string())
    );
}

#[test]
fn fee_rate_parse_success_keeps_fees_complete() {
    let mut completeness = AccountDataCompleteness::default();
    let rates = fee_rates_from_best_effort_value(
        Ok(serde_json::json!({
            "userCrossRate": "0.0004",
            "userAddRate": "0.0001"
        })),
        &mut completeness,
    );

    assert!(rates.rate_for(false, false).is_some());
    assert_eq!(completeness.section_warning(AccountDataSection::Fees), None);
}

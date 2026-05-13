use super::{AccountData, AccountDataCompleteness, AccountDataSection, parse_account_number};
use crate::account::types::{ClearinghouseState, MarginSummary, SpotClearinghouseState};

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

#[test]
fn account_number_parser_rejects_invalid_or_nonfinite_values() {
    assert_eq!(parse_account_number(" 123.45 "), Some(123.45));
    assert_eq!(parse_account_number("-0.5"), Some(-0.5));

    assert_eq!(parse_account_number("bad"), None);
    assert_eq!(parse_account_number("NaN"), None);
    assert_eq!(parse_account_number("inf"), None);
}

#[test]
fn position_action_snapshot_is_fresh_only_within_cutoff() {
    let data = AccountData {
        fetched_at_ms: 1_000,
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "0".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "0".to_string(),
            asset_positions: Vec::new(),
        },
        spot: SpotClearinghouseState {
            balances: Vec::new(),
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: Default::default(),
        completeness: Default::default(),
    };

    assert!(data.is_fresh_for_position_action(1_000 + AccountData::POSITION_ACTION_MAX_AGE_MS));
    assert!(
        !data.is_fresh_for_position_action(1_000 + AccountData::POSITION_ACTION_MAX_AGE_MS + 1)
    );
    assert!(!data.is_fresh_for_position_action(999));
}

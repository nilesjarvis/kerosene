use super::{
    AccountData, AccountDataCompleteness, AccountDataFetchScope, AccountDataSection,
    parse_account_number,
};
use crate::account::types::{
    AccountAbstractionMode, ClearinghouseState, MarginSummary, SpotBalance, SpotClearinghouseState,
};

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
fn selected_hip3_fetch_scope_reduces_estimated_request_weight() {
    let all_markets = AccountDataFetchScope::all_markets(["xyz", "flx", "new"]);
    let selected = AccountDataFetchScope::hip3_dex("XYZ");

    assert_eq!(selected.selected_hip3_dex(), Some("xyz"));
    assert!(selected.estimated_info_weight() < all_markets.estimated_info_weight());
    assert!(!selected.fetches_main_open_orders());
    assert_eq!(
        all_markets.hip3_dexes(&[]),
        vec!["flx".to_string(), "new".to_string(), "xyz".to_string()]
    );
}

#[test]
fn automatic_refresh_interval_increases_with_heavier_scope() {
    let all_markets = AccountDataFetchScope::all_markets(["xyz", "flx", "new"]);
    let selected = AccountDataFetchScope::hip3_dex("XYZ");

    assert!(
        all_markets.automatic_refresh_interval_secs() > selected.automatic_refresh_interval_secs()
    );
}

fn account_data_for_available_margin(
    abstraction: AccountAbstractionMode,
    portfolio_margin_enabled: bool,
) -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        fetched_at_ms: 1_000,
        account_abstraction: abstraction,
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "100".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "25".to_string(),
            asset_positions: Vec::new(),
        },
        clearinghouses_by_dex: std::collections::HashMap::new(),
        spot: SpotClearinghouseState {
            balances: vec![
                SpotBalance {
                    coin: "USDC".to_string(),
                    token: Some(0),
                    total: "90".to_string(),
                    hold: "10".to_string(),
                    entry_ntl: "0".to_string(),
                    supplied: None,
                },
                SpotBalance {
                    coin: "USDH".to_string(),
                    token: Some(360),
                    total: "30".to_string(),
                    hold: "5".to_string(),
                    entry_ntl: "0".to_string(),
                    supplied: None,
                },
            ],
            portfolio_margin_enabled,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: Some(vec![
                (0, "55".to_string()),
                (360, "22".to_string()),
            ]),
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: Default::default(),
        completeness: Default::default(),
    }
}

#[test]
fn portfolio_margin_available_uses_after_maintenance_value() {
    let data = account_data_for_available_margin(AccountAbstractionMode::PortfolioMargin, true);

    assert_eq!(data.available_margin_usdc(), Some(55.0));
    assert_eq!(data.available_margin_for_token(360), Some(22.0));
}

#[test]
fn unified_available_uses_spot_balance_after_holds() {
    let data = account_data_for_available_margin(AccountAbstractionMode::UnifiedAccount, false);

    assert_eq!(data.available_margin_usdc(), Some(80.0));
    assert_eq!(data.available_margin_for_token(360), Some(25.0));
}

#[test]
fn dex_abstraction_available_keeps_nonzero_spot_fallback() {
    let data = account_data_for_available_margin(AccountAbstractionMode::DexAbstraction, false);

    assert_eq!(data.available_margin_usdc(), Some(80.0));
}

#[test]
fn default_abstraction_keeps_nonzero_spot_fallback() {
    let data = account_data_for_available_margin(AccountAbstractionMode::Default, false);

    assert_eq!(data.available_margin_usdc(), Some(80.0));
}

#[test]
fn unknown_abstraction_fails_closed_for_available_margin() {
    let data = account_data_for_available_margin(
        AccountAbstractionMode::Unknown("unavailable".to_string()),
        false,
    );

    assert_eq!(data.available_margin_usdc(), None);
    assert_eq!(data.available_margin_for_token(360), None);
}

#[test]
fn position_action_snapshot_is_fresh_only_within_cutoff() {
    let data = AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        fetched_at_ms: 1_000,
        account_abstraction: Default::default(),
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
        clearinghouses_by_dex: std::collections::HashMap::new(),
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

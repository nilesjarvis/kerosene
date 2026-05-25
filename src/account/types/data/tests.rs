use super::{
    AccountData, AccountDataCompleteness, AccountDataFetchScope, AccountDataSection,
    parse_account_number,
};
use crate::account::types::{
    AccountAbstractionMode, ClearinghouseState, MarginSummary, SpotBalance, SpotClearinghouseState,
};

mod completeness;
mod fetch_scope;
mod freshness;
mod margin;
mod parsing;

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

fn account_data_snapshot(fetched_at_ms: u64) -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        fetched_at_ms,
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
    }
}

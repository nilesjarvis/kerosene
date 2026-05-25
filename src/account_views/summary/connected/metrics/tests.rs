use super::*;
use crate::account::{
    AccountAbstractionMode, AccountData, ClearinghouseState, MarginSummary, SpotBalance,
    SpotClearinghouseState,
};
use crate::config::MarketUniverseConfig;

mod calculations;
mod formatting;
mod shared_balance;

fn account_data_for_shared_total(
    abstraction: AccountAbstractionMode,
    portfolio_margin_enabled: bool,
) -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: abstraction,
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "92".to_string(),
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
            balances: vec![SpotBalance {
                coin: "USDC".to_string(),
                token: Some(0),
                total: "100".to_string(),
                hold: "0".to_string(),
                entry_ntl: "0".to_string(),
                supplied: None,
            }],
            portfolio_margin_enabled,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: Default::default(),
        completeness: Default::default(),
        fetched_at_ms: 1,
    }
}

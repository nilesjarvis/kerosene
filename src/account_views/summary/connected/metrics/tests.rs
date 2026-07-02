use super::*;
use crate::account::{
    AccountAbstractionMode, AccountData, ClearinghouseState, MarginSummary, SpotBalance,
    SpotClearinghouseState,
};
use crate::api::{ExchangeSymbol, MarketType};
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

#[test]
fn connected_total_values_spot_only_token_at_its_spot_pair_mid() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.muted_tickers.clear();
    terminal.exchange_symbols = vec![ExchangeSymbol {
        key: "@77".to_string(),
        ticker: "JEFF".to_string(),
        category: "spot".to_string(),
        display_name: Some("JEFF/USDC".to_string()),
        keywords: Vec::new(),
        asset_index: 10_077,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        market_type: MarketType::Spot,
        outcome: None,
    }];
    terminal.all_mids.insert("@77".to_string(), 2.0);
    terminal
        .all_mids_updated_at_ms
        .insert("@77".to_string(), TradingTerminal::now_ms());

    let mut data = account_data_for_shared_total(AccountAbstractionMode::Disabled, false);
    data.clearinghouse.margin_summary.account_value = "0".to_string();
    data.spot.balances = vec![SpotBalance {
        coin: "JEFF".to_string(),
        token: None,
        total: "10".to_string(),
        hold: "0".to_string(),
        entry_ntl: "5".to_string(),
        supplied: None,
    }];

    let values = terminal.connected_summary_values(&data);

    // The balance coin "JEFF" is not a mids key; the header total must mark
    // the holding through its "@77" spot pair, not freeze at cost.
    assert_eq!(values.total_value, "20.00");
}

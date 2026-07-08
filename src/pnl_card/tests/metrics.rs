use crate::account::{
    AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary, SpotBalance,
    SpotClearinghouseState, UserFeeRates,
};
use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::app_state::TradingTerminal;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

fn set_connected_account_data(terminal: &mut TradingTerminal, data: AccountData) {
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, data);
}

fn outcome_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: "OUT95-YES".to_string(),
        category: "outcome".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 100_000_950,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 95,
            question_id: None,
            question_name: Some("Will BTC close green?".to_string()),
            question_description: None,
            question_class: None,
            question_underlying: None,
            question_expiry: None,
            question_price_thresholds: Vec::new(),
            question_period: None,
            question_named_outcomes: Vec::new(),
            question_settled_named_outcomes: Vec::new(),
            question_fallback_outcome: None,
            bucket_index: None,
            is_question_fallback: false,
            side_index: 0,
            side_name: "Yes".to_string(),
            outcome_name: "Recurring".to_string(),
            description: "Will BTC close green?".to_string(),
            class: None,
            underlying: None,
            expiry: None,
            target_price: None,
            period: None,
            quote_symbol: "USDC".to_string(),
            quote_token_index: Some(crate::api::USDC_TOKEN_INDEX),
            encoding: 950,
        }),
    }
}

fn outcome_account_data() -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
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
            balances: vec![SpotBalance {
                coin: "+950".to_string(),
                token: None,
                total: "30".to_string(),
                hold: "0".to_string(),
                entry_ntl: "12".to_string(),
                supplied: None,
            }],
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: UserFeeRates::default(),
        completeness: AccountDataCompleteness::default(),
        fetched_at_ms: 1_000,
    }
}

#[test]
fn outcome_balance_position_resolves_metrics_with_market_label() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![outcome_symbol("#950")];
    set_connected_account_data(&mut terminal, outcome_account_data());

    let Some(metrics) = terminal.position_pnl_card_metrics("#950") else {
        panic!("outcome balance should synthesize an open position for the PnL card");
    };

    assert_eq!(metrics.ticker, "YES: Will BTC close green?");
    assert_eq!(metrics.context, "Long 30");
}

#[test]
fn summary_pnl_card_metrics_include_outcome_balance_positions() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![outcome_symbol("#950")];
    set_connected_account_data(&mut terminal, outcome_account_data());

    let Ok(metrics) = terminal.summary_pnl_card_metrics() else {
        panic!("summary PnL card should include synthesized outcome positions");
    };

    assert_eq!(metrics.ticker, "PORTFOLIO");
    assert_eq!(metrics.context, "1 open position");
    assert_eq!(metrics.upnl, 0.0);
    assert_eq!(metrics.asset_move_pct, Some(0.0));
    assert_eq!(metrics.leveraged_pct, Some(0.0));
}

#[test]
fn summary_pnl_card_metrics_refuse_positions_with_unavailable_pnl() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![
        outcome_symbol("#950"),
        ExchangeSymbol {
            key: "@142".to_string(),
            ticker: "UBTC".to_string(),
            category: "spot".to_string(),
            display_name: Some("UBTC/USDC".to_string()),
            keywords: Vec::new(),
            asset_index: 10_142,
            collateral_token: None,
            sz_decimals: 5,
            max_leverage: 1,
            only_isolated: false,
            market_type: MarketType::Spot,
            outcome: None,
        },
    ];
    let mut data = outcome_account_data();
    // A spot balance without an entry notional or reconciling fills has no
    // knowable PnL; the portfolio card must refuse rather than render a
    // total that silently excludes it.
    data.spot.balances.push(SpotBalance {
        coin: "UBTC".to_string(),
        token: None,
        total: "2".to_string(),
        hold: "0".to_string(),
        entry_ntl: "0".to_string(),
        supplied: None,
    });
    set_connected_account_data(&mut terminal, data);

    let Err(message) = terminal.summary_pnl_card_metrics() else {
        panic!("portfolio card must not total positions with unknown PnL");
    };

    assert!(message.contains("unavailable"));
}

#[test]
fn pnl_card_position_lookup_misses_unknown_coins() {
    let mut terminal = TradingTerminal::boot().0;
    set_connected_account_data(&mut terminal, outcome_account_data());

    assert!(terminal.pnl_card_position_for_coin("#999").is_none());
}

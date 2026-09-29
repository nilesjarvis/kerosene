use super::*;
use crate::account::{
    AccountData, AccountDataCompleteness, AssetPosition, ClearinghouseState, MarginSummary,
    Position, PositionLeverage, SpotClearinghouseState, UserFeeRates,
};
use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::app_state::sensitive_string;
use crate::config::AccountProfile;
use crate::order_execution::PendingOrderAction;
use crate::signing::OrderKind;

mod positions;
mod trading;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

#[derive(Debug, Clone, PartialEq, Eq)]
struct TicketSnapshot {
    active_symbol: String,
    active_symbol_display: String,
    order_kind: OrderKind,
    order_quantity: String,
    order_quantity_is_usd: bool,
    order_price: String,
    presets_menu_expanded: bool,
    alfred_open: bool,
}

fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 5,
        max_leverage: 50,
        only_isolated: false,
        growth_mode: false,
        market_type,
        outcome: None,
    }
}

fn spot_symbol(key: &str, ticker: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        ticker: ticker.to_string(),
        category: "spot".to_string(),
        display_name: Some(format!("{ticker}/USDC")),
        max_leverage: 1,
        ..symbol(key, MarketType::Spot)
    }
}

fn outcome_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 66,
            contract: crate::api::OutcomeContract::verified_fixture(),
            venue: None,
            question_id: Some(12),
            question_name: Some("Recurring".to_string()),
            question_description: None,
            question_class: Some("priceBucket".to_string()),
            question_underlying: Some("BTC".to_string()),
            question_expiry: Some("20260520-0600".to_string()),
            question_price_thresholds: vec!["75348".to_string(), "78423".to_string()],
            question_period: Some("1d".to_string()),
            question_named_outcomes: vec![67, 68, 69],
            question_settled_named_outcomes: Vec::new(),
            question_fallback_outcome: Some(66),
            bucket_index: Some(0),
            is_question_fallback: false,
            side_index: 0,
            side_name: "Yes".to_string(),
            outcome_name: "Recurring Named Outcome".to_string(),
            description: "index:0".to_string(),
            class: None,
            underlying: None,
            expiry: None,
            target_price: None,
            period: None,
            quote_symbol: "USDH".to_string(),
            quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
            encoding: 660,
        }),
        ..symbol(key, MarketType::Outcome)
    }
}

fn account_data_with_position(coin: &str, fetched_at_ms: u64) -> AccountData {
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
            asset_positions: vec![AssetPosition {
                position: Position {
                    coin: coin.to_string(),
                    szi: "1".to_string(),
                    entry_px: "100".to_string(),
                    position_value: "100".to_string(),
                    unrealized_pnl: "0".to_string(),
                    liquidation_px: None,
                    leverage: PositionLeverage {
                        leverage_type: "cross".to_string(),
                        value: 1,
                    },
                    margin_used: "0".to_string(),
                    cum_funding: None,
                },
                liquidation_px: None,
            }],
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
        fee_rates: UserFeeRates::default(),
        completeness: AccountDataCompleteness::default(),
        fetched_at_ms,
    }
}

fn connect_test_account(terminal: &mut TradingTerminal) {
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.wallet_address_input = TEST_ACCOUNT.to_string();
    terminal.accounts = vec![AccountProfile {
        master_address: None,
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: TEST_ACCOUNT.to_string(),
        agent_key: sensitive_string("").into_zeroizing(),
        hydromancer_api_key: sensitive_string("").into_zeroizing(),
    }];
    terminal.active_account_index = 0;
    terminal.set_committed_agent_key_for_test("agent-key");
}

fn alfred_close_terminal(fetched_at_ms: u64) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        account_data_with_position("BTC", fetched_at_ms),
    );
    terminal.account_loading = false;
    terminal.alfred.open = true;
    terminal.alfred.query = "close BTC".to_string();
    terminal.close_menu_coin = Some("BTC".to_string());
    terminal
}

fn alfred_trade_terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![
        symbol("BTC", MarketType::Perp),
        symbol("ETH", MarketType::Perp),
    ];
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.order_kind = OrderKind::Limit;
    terminal.order_quantity = "old-size".to_string();
    terminal.order_quantity_is_usd = true;
    terminal.order_price = "old-price".to_string();
    terminal.presets_menu_expanded = true;
    terminal.alfred.open = true;
    terminal
}

fn add_mid(terminal: &mut TradingTerminal, symbol: &str, mid: f64) {
    terminal.all_mids.insert(symbol.to_string(), mid);
    terminal
        .all_mids_updated_at_ms
        .insert(symbol.to_string(), TradingTerminal::now_ms());
}

fn ticket_snapshot(terminal: &TradingTerminal) -> TicketSnapshot {
    TicketSnapshot {
        active_symbol: terminal.active_symbol.clone(),
        active_symbol_display: terminal.active_symbol_display.clone(),
        order_kind: terminal.order_kind,
        order_quantity: terminal.order_quantity.clone(),
        order_quantity_is_usd: terminal.order_quantity_is_usd,
        order_price: terminal.order_price.clone(),
        presets_menu_expanded: terminal.presets_menu_expanded,
        alfred_open: terminal.alfred.open,
    }
}

fn order_status_or_panic(terminal: &TradingTerminal) -> (&str, bool) {
    match terminal.order_status.as_ref() {
        Some((message, is_error)) => (message.as_str(), *is_error),
        None => panic!("missing order status"),
    }
}

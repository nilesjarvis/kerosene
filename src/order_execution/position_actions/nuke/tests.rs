use super::planning::{
    NukePlan, NukePositionClassification, NukePositionInput, NukePositionOrder, NukeSkipReason,
    NukeSymbolInfo, build_nuke_position_order, classify_nuke_position, parse_nuke_position_size,
    plan_nuke_positions_from_inputs,
};
use crate::account::{
    AccountData, AccountDataCompleteness, AccountDataSection, AssetPosition, ClearinghouseState,
    MarginSummary, Position, PositionLeverage, SpotClearinghouseState, UserFeeRates,
};
use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::config::AccountProfile;
use crate::order_execution::pricing::DEFAULT_MARKET_SLIPPAGE_PCT;

mod classification;
mod execution;
mod orders;
mod planning;

const DEFAULT_MARKET_SLIPPAGE: f64 = DEFAULT_MARKET_SLIPPAGE_PCT / 100.0;
const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

fn connect_test_account(terminal: &mut TradingTerminal) {
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.wallet_address_input = TEST_ACCOUNT.to_string();
    terminal.accounts = vec![AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: TEST_ACCOUNT.to_string(),
        agent_key: sensitive_string("").into_zeroizing(),
        hydromancer_api_key: sensitive_string("").into_zeroizing(),
    }];
    terminal.active_account_index = 0;
}

fn perp_sym() -> NukeSymbolInfo {
    NukeSymbolInfo {
        asset_index: 7,
        sz_decimals: 4,
        market_type: MarketType::Perp,
    }
}

fn nuke_input(
    coin: &str,
    raw_size: &str,
    is_hidden: bool,
    sym: Option<NukeSymbolInfo>,
    mid: Option<f64>,
) -> NukePositionInput {
    NukePositionInput {
        coin: coin.to_string(),
        raw_size: raw_size.to_string(),
        is_hidden,
        sym,
        mid,
    }
}

fn stale_account_data() -> AccountData {
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
        fetched_at_ms: 1,
    }
}

fn active_position(coin: &str, szi: &str) -> AssetPosition {
    AssetPosition {
        position: Position {
            coin: coin.to_string(),
            szi: szi.to_string(),
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
    }
}

fn exchange_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 7,
        collateral_token: None,
        sz_decimals: 4,
        max_leverage: 50,
        only_isolated: false,
        market_type: MarketType::Perp,
        outcome: None,
    }
}

fn terminal_with_stale_account() -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    connect_test_account(&mut terminal);
    terminal.set_committed_agent_key_for_test("agent-key");
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, stale_account_data());
    terminal.account_loading = false;
    terminal
}

fn terminal_with_incomplete_fresh_account() -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    connect_test_account(&mut terminal);
    terminal.set_committed_agent_key_for_test("agent-key");
    terminal.exchange_symbols = vec![exchange_symbol("BTC")];
    let now_ms = TradingTerminal::now_ms();
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), now_ms);
    let mut account_data = stale_account_data();
    account_data.fetched_at_ms = now_ms;
    account_data.clearinghouse.asset_positions = vec![active_position("BTC", "1")];
    account_data
        .completeness
        .mark_incomplete(AccountDataSection::Positions, "HIP-3 positions unavailable");
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, account_data);
    terminal.account_loading = false;
    terminal
}

fn order_or_panic(order: Option<NukePositionOrder>, context: &str) -> NukePositionOrder {
    match order {
        Some(order) => order,
        None => panic!("{context}"),
    }
}

fn plan_or_panic(result: Result<NukePlan, String>, context: &str) -> NukePlan {
    match result {
        Ok(plan) => plan,
        Err(error) => panic!("{context}: {error}"),
    }
}

fn plan_error_or_panic(result: Result<NukePlan, String>, context: &str) -> String {
    match result {
        Ok(plan) => panic!("{context}: expected error, got {plan:?}"),
        Err(error) => error,
    }
}

fn order_status_or_panic(terminal: &TradingTerminal) -> (&str, bool) {
    match &terminal.order_status {
        Some((message, is_error)) => (message.as_str(), *is_error),
        None => panic!("status"),
    }
}

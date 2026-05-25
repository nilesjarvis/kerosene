use super::planning::{
    NukePlan, NukePositionClassification, NukePositionInput, NukePositionOrder, NukeSkipReason,
    NukeSymbolInfo, build_nuke_position_order, classify_nuke_position, parse_nuke_position_size,
    plan_nuke_positions_from_inputs,
};
use crate::account::{
    AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary,
    SpotClearinghouseState, UserFeeRates,
};
use crate::api::MarketType;
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::order_execution::pricing::DEFAULT_MARKET_SLIPPAGE_PCT;

mod classification;
mod execution;
mod orders;
mod planning;

const DEFAULT_MARKET_SLIPPAGE: f64 = DEFAULT_MARKET_SLIPPAGE_PCT / 100.0;

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
    is_visible: bool,
    sym: Option<NukeSymbolInfo>,
    mid: Option<f64>,
) -> NukePositionInput {
    NukePositionInput {
        coin: coin.to_string(),
        raw_size: raw_size.to_string(),
        is_visible,
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

fn terminal_with_stale_account() -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.wallet_key_input = sensitive_string("agent-key");
    terminal.account_data = Some(stale_account_data());
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

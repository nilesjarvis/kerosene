use super::*;
use crate::account::{
    AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary, OpenOrder,
    SpotClearinghouseState, UserFeeRates,
};
use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::chart::OrderOverlay;
use crate::chart_state::ChartInstance;
use crate::config::AccountProfile;
use crate::signing::{ChaseOrder, OrderKind};
use crate::timeframe::Timeframe;

mod overlays;
mod start;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

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
        market_type,
        outcome: None,
    }
}

fn fallback_outcome_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 66,
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
            bucket_index: None,
            is_question_fallback: true,
            side_index: 0,
            side_name: "Yes".to_string(),
            outcome_name: "Recurring Fallback".to_string(),
            description: "other".to_string(),
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

fn account_profile() -> AccountProfile {
    AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: TEST_ACCOUNT.to_string(),
        agent_key: "agent-key".to_string().into(),
        hydromancer_api_key: String::new().into(),
    }
}

fn chase_ready_terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.accounts = vec![account_profile()];
    terminal.active_account_index = 0;
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.set_committed_agent_key_for_test("agent-key");
    terminal.wallet_address_input = TEST_ACCOUNT.to_string();
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.order_kind = OrderKind::Chase;
    terminal.order_quantity = "2.5".to_string();
    terminal.order_quantity_is_usd = false;
    terminal.pending_order_action = None;
    terminal
}

fn open_order(oid: u64) -> OpenOrder {
    OpenOrder {
        coin: "BTC".to_string(),
        side: "B".to_string(),
        limit_px: "100".to_string(),
        sz: "1".to_string(),
        oid,
        timestamp: 1,
        reduce_only: Some(false),
        is_trigger: None,
        order_type: None,
        tif: None,
        trigger_px: None,
    }
}

fn account_data_with_order(order: OpenOrder) -> AccountData {
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
        open_orders: vec![order],
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: UserFeeRates::default(),
        completeness: AccountDataCompleteness::default(),
        fetched_at_ms: TradingTerminal::now_ms(),
    }
}

fn selected_chase(terminal: &TradingTerminal) -> &ChaseOrder {
    match terminal.selected_chase() {
        Some(chase) => chase,
        None => panic!("chase order should be inserted"),
    }
}

fn selected_chase_id(terminal: &TradingTerminal) -> u64 {
    match terminal.selected_chase_id() {
        Some(chase_id) => chase_id,
        None => panic!("resting chase should be selected"),
    }
}

fn chart_instance(terminal: &TradingTerminal, chart_id: u64) -> &ChartInstance {
    match terminal.charts.get(&chart_id) {
        Some(instance) => instance,
        None => panic!("chart instance {chart_id}"),
    }
}

fn chart_instance_mut(terminal: &mut TradingTerminal, chart_id: u64) -> &mut ChartInstance {
    match terminal.charts.get_mut(&chart_id) {
        Some(instance) => instance,
        None => panic!("chart instance {chart_id}"),
    }
}

fn order_status_error_contains(terminal: &TradingTerminal, needle: &str) -> bool {
    terminal
        .order_status
        .as_ref()
        .is_some_and(|(message, is_error)| *is_error && message.contains(needle))
}

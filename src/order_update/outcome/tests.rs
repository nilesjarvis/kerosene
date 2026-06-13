use super::*;
use crate::account::{
    AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary, SpotBalance,
    SpotClearinghouseState,
};
use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::order_execution::PendingOrderAction;
use crate::signing::OrderKind;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";
const OTHER_ACCOUNT: &str = "0xdef0000000000000000000000000000000000000";

fn balance(total: &str, hold: &str) -> SpotBalance {
    SpotBalance {
        coin: "+650".to_string(),
        token: None,
        total: total.to_string(),
        hold: hold.to_string(),
        entry_ntl: "0".to_string(),
        supplied: None,
    }
}

fn outcome_symbol() -> ExchangeSymbol {
    ExchangeSymbol {
        key: "#650".to_string(),
        ticker: "OUT650-YES".to_string(),
        category: "outcome".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 100_000_000,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 650,
            question_id: Some(650),
            question_name: Some("Test outcome".to_string()),
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
            side_name: "YES".to_string(),
            outcome_name: "YES".to_string(),
            description: "Test outcome".to_string(),
            class: None,
            underlying: None,
            expiry: None,
            target_price: None,
            period: None,
            quote_symbol: "USDC".to_string(),
            quote_token_index: None,
            encoding: 650,
        }),
    }
}

fn account_data_with_spot_balance(balance: SpotBalance) -> AccountData {
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
            balances: vec![balance],
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: Default::default(),
        completeness: AccountDataCompleteness::default(),
        fetched_at_ms: 1,
    }
}

fn seed_existing_ticket(terminal: &mut TradingTerminal) {
    terminal.order_kind = OrderKind::Market;
    terminal.order_quantity = "5".to_string();
    terminal.order_price = "0.42".to_string();
    terminal.order_quantity_is_usd = true;
    terminal.order_percentage = 25.0;
}

fn assert_existing_ticket_unchanged(terminal: &TradingTerminal) {
    assert_eq!(terminal.order_kind, OrderKind::Market);
    assert_eq!(terminal.order_quantity, "5");
    assert_eq!(terminal.order_price, "0.42");
    assert!(terminal.order_quantity_is_usd);
    assert_eq!(terminal.order_percentage, 25.0);
}

fn prepare_outcome_preset_context(terminal: &mut TradingTerminal) {
    terminal.active_symbol = "#650".to_string();
    terminal.exchange_symbols = vec![outcome_symbol()];
    terminal.all_mids.insert("#650".to_string(), 0.42);
    terminal
        .all_mids_updated_at_ms
        .insert("#650".to_string(), TradingTerminal::now_ms());
    terminal.presets_menu_expanded = true;
}

#[test]
fn available_outcome_contracts_floor_available_balance() {
    assert_eq!(
        outcome_available_contracts(&balance("10.9", "0.2")),
        Some(10.0)
    );
    assert_eq!(outcome_available_contracts(&balance("1.9", "1.0")), None);
    assert_eq!(outcome_available_contracts(&balance("bad", "0")), None);
    assert_eq!(outcome_available_contracts(&balance("inf", "0")), None);
}

#[test]
fn outcome_chase_preset_is_rejected_without_mutating_order_ticket() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.order_kind = OrderKind::Market;
    terminal.order_quantity = "5".to_string();
    terminal.order_price = "0.42".to_string();
    terminal.presets_menu_expanded = true;

    let preset = OrderPreset {
        label: "Chase".to_string(),
        size: 100.0,
        price_offset_pct: Some(1.0),
    };
    let _ = terminal.handle_execute_outcome_preset(OrderKind::Chase, preset, true);

    assert_eq!(terminal.order_kind, OrderKind::Market);
    assert_eq!(terminal.order_quantity, "5");
    assert_eq!(terminal.order_price, "0.42");
    assert!(!terminal.presets_menu_expanded);
    assert_eq!(
        terminal.order_status,
        Some(("Outcome automation is not supported yet".to_string(), true))
    );
}

#[test]
fn outcome_sell_prefill_ignores_stale_account_snapshot() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.account_data_address = Some(OTHER_ACCOUNT.to_string());
    terminal.account_data = Some(account_data_with_spot_balance(balance("10", "0")));
    terminal.exchange_symbols = vec![outcome_symbol()];
    seed_existing_ticket(&mut terminal);

    let _task = terminal.handle_prefill_outcome_sell("+650".to_string());

    assert_existing_ticket_unchanged(&terminal);
    assert_eq!(
        terminal.order_status,
        Some(("No available outcome contracts to sell".to_string(), true))
    );
}

#[test]
fn outcome_sell_prefill_waits_for_loading_account_refresh_without_mutating_ticket() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.account_data_address = Some(TEST_ACCOUNT.to_string());
    terminal.account_data = Some(account_data_with_spot_balance(balance("10", "0")));
    terminal.account_loading = true;
    terminal.exchange_symbols = vec![outcome_symbol()];
    seed_existing_ticket(&mut terminal);

    let _task = terminal.handle_prefill_outcome_sell("+650".to_string());

    assert_existing_ticket_unchanged(&terminal);
    assert_eq!(
        terminal.order_status,
        Some((
            "Account refresh in progress; wait for fresh account data before prefilling outcome sell"
                .to_string(),
            true,
        ))
    );
}

#[test]
fn outcome_sell_prefill_waits_for_reconciled_account_without_mutating_ticket() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.account_data_address = Some(TEST_ACCOUNT.to_string());
    terminal.account_data = Some(account_data_with_spot_balance(balance("10", "0")));
    terminal.account_reconciliation_required = true;
    terminal.exchange_symbols = vec![outcome_symbol()];
    seed_existing_ticket(&mut terminal);

    let _task = terminal.handle_prefill_outcome_sell("+650".to_string());

    assert_existing_ticket_unchanged(&terminal);
    assert_eq!(
        terminal.order_status,
        Some((
            "Account refresh pending; wait for fresh account data before prefilling outcome sell"
                .to_string(),
            true,
        ))
    );
}

#[test]
fn outcome_sell_prefill_rejects_stale_same_account_snapshot_without_mutating_ticket() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.account_data_address = Some(TEST_ACCOUNT.to_string());
    terminal.account_data = Some(account_data_with_spot_balance(balance("10", "0")));
    terminal.exchange_symbols = vec![outcome_symbol()];
    seed_existing_ticket(&mut terminal);

    let _task = terminal.handle_prefill_outcome_sell("+650".to_string());

    assert_existing_ticket_unchanged(&terminal);
    assert_eq!(
        terminal.order_status,
        Some((
            "Account data is stale for outcome sell prefill; refresh before selling outcome contracts"
                .to_string(),
            true,
        ))
    );
}

#[test]
fn outcome_market_preset_pending_request_does_not_mutate_order_ticket() {
    let mut terminal = TradingTerminal::boot().0;
    prepare_outcome_preset_context(&mut terminal);
    seed_existing_ticket(&mut terminal);
    terminal.pending_order_action = Some(PendingOrderAction::Buy);

    let _task = terminal.handle_execute_preset(
        OrderKind::Market,
        OrderPreset {
            label: "10".to_string(),
            size: 10.0,
            price_offset_pct: None,
        },
        true,
    );

    assert_existing_ticket_unchanged(&terminal);
    assert!(terminal.presets_menu_expanded);
    assert_eq!(
        terminal.order_status,
        Some((
            "Wait for pending trading requests to finish before placing an order".to_string(),
            true,
        ))
    );
}

#[test]
fn outcome_limit_preset_reconciliation_required_does_not_mutate_order_ticket() {
    let mut terminal = TradingTerminal::boot().0;
    prepare_outcome_preset_context(&mut terminal);
    seed_existing_ticket(&mut terminal);
    terminal.account_reconciliation_required = true;

    let _task = terminal.handle_execute_preset(
        OrderKind::Limit,
        OrderPreset {
            label: "10".to_string(),
            size: 10.0,
            price_offset_pct: Some(1.0),
        },
        true,
    );

    assert_existing_ticket_unchanged(&terminal);
    assert!(terminal.presets_menu_expanded);
    assert_eq!(
        terminal.order_status,
        Some((
            "Account refresh pending; wait for fresh account data before placing an order"
                .to_string(),
            true,
        ))
    );
}

#[test]
fn outcome_market_preset_missing_signing_context_does_not_mutate_order_ticket() {
    let mut terminal = TradingTerminal::boot().0;
    prepare_outcome_preset_context(&mut terminal);
    seed_existing_ticket(&mut terminal);

    let _task = terminal.handle_execute_preset(
        OrderKind::Market,
        OrderPreset {
            label: "10".to_string(),
            size: 10.0,
            price_offset_pct: None,
        },
        true,
    );

    assert_existing_ticket_unchanged(&terminal);
    assert!(terminal.presets_menu_expanded);
    assert_eq!(
        terminal.order_status,
        Some(("Connect wallet and enter agent key first".to_string(), true))
    );
}

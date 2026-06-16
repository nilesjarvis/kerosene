use crate::account::{
    AccountData, AccountDataCompleteness, AccountDataSection, AssetPosition, ClearinghouseState,
    MarginSummary, Position, PositionLeverage, SpotClearinghouseState, UserFeeRates,
};
use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::config::AccountProfile;
use crate::order_execution::{
    OneShotPlacementContext, OrderSurface, PendingOrderAction, PreparedExchangeOrder,
    TicketOrderPlaceIntent,
};
use crate::order_update::PendingOneShotStatusRequest;
use crate::signing::{ExchangeOrderKind, OrderKind};

mod outcomes;

fn outcome_info(is_question_fallback: bool) -> OutcomeSymbolInfo {
    OutcomeSymbolInfo {
        outcome_id: 65,
        question_id: Some(12),
        question_name: Some("Recurring".to_string()),
        question_description: Some(
            "class:priceBucket|underlying:BTC|expiry:20260520-0600".to_string(),
        ),
        question_class: Some("priceBucket".to_string()),
        question_underlying: Some("BTC".to_string()),
        question_expiry: Some("20260520-0600".to_string()),
        question_price_thresholds: vec!["75348".to_string(), "78423".to_string()],
        question_period: Some("1d".to_string()),
        question_named_outcomes: vec![67, 68, 69],
        question_settled_named_outcomes: Vec::new(),
        question_fallback_outcome: Some(66),
        bucket_index: Some(0),
        is_question_fallback,
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
        encoding: 650,
    }
}

fn outcome_symbol(key: &str, asset_index: u32, is_question_fallback: bool) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: "OUT65-YES".to_string(),
        category: "outcome".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        market_type: MarketType::Outcome,
        outcome: Some(outcome_info(is_question_fallback)),
    }
}

fn terminal_for_outcome_order(symbol: ExchangeSymbol) -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    let account = "0xabc0000000000000000000000000000000000000";
    terminal.connected_address = Some(account.to_string());
    terminal.wallet_address_input = account.to_string();
    terminal.accounts = vec![AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: account.to_string(),
        agent_key: sensitive_string("").into_zeroizing(),
        hydromancer_api_key: sensitive_string("").into_zeroizing(),
    }];
    terminal.active_account_index = 0;
    terminal.set_committed_agent_key_for_test("agent-key");
    terminal.active_symbol = symbol.key.clone();
    terminal.exchange_symbols = vec![symbol];
    terminal.order_kind = OrderKind::Limit;
    terminal.order_price = "0.42123456".to_string();
    terminal.order_quantity = "3".to_string();
    terminal.order_quantity_is_usd = true;
    terminal.order_reduce_only = true;
    terminal
        .all_mids
        .insert(terminal.active_symbol.clone(), 0.42);
    terminal
        .all_mids_updated_at_ms
        .insert(terminal.active_symbol.clone(), TradingTerminal::now_ms());
    terminal
}

fn first_symbol_or_panic(terminal: &TradingTerminal) -> &ExchangeSymbol {
    match terminal.exchange_symbols.first() {
        Some(symbol) => symbol,
        None => panic!("missing symbol"),
    }
}

fn prepared_order_or_panic(
    terminal: &TradingTerminal,
    symbol: &ExchangeSymbol,
    is_buy: bool,
) -> PreparedExchangeOrder {
    let order_kind = ExchangeOrderKind::try_from(terminal.order_kind)
        .expect("test order kind should be exchange order kind");
    let intent = TradingTerminal::ticket_order_place_intent(TicketOrderPlaceIntent {
        surface: crate::order_execution::OrderSurface::Ticket,
        symbol_key: symbol.key.clone(),
        is_buy,
        order_kind,
        price_input: terminal.order_price.clone(),
        quantity_input: terminal.order_quantity.clone(),
        quantity_is_usd: terminal.order_quantity_is_usd,
        reduce_only: terminal.order_reduce_only,
    });

    match terminal.prepare_place_order(intent) {
        Ok(prepared) => prepared,
        Err(error) => panic!("valid outcome order: {error}"),
    }
}

fn order_status_or_panic(terminal: &TradingTerminal) -> (&str, bool) {
    match terminal.order_status.as_ref() {
        Some((message, is_error)) => (message.as_str(), *is_error),
        None => panic!("missing order status"),
    }
}

fn pending_one_shot_status_request() -> PendingOneShotStatusRequest {
    PendingOneShotStatusRequest::new(
        7,
        &OneShotPlacementContext {
            account_address: "0xabc0000000000000000000000000000000000000".to_string(),
            cloid: "0x00000000000000000000000000000001".to_string(),
            surface: OrderSurface::Ticket,
            symbol_key: "BTC".to_string(),
            order_kind: ExchangeOrderKind::Market,
        },
    )
}

#[test]
fn execute_order_rejects_while_order_action_pending() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.order_kind = OrderKind::Market;
    terminal.pending_order_action = Some(PendingOrderAction::Buy);

    let _task = terminal.execute_order(true);

    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(
        status,
        "Wait for pending trading requests to finish before placing an order"
    );
    assert_eq!(
        terminal.pending_order_action,
        Some(PendingOrderAction::Buy),
        "status: {:?}",
        terminal.order_status
    );
}

#[test]
fn execute_order_rejects_while_one_shot_status_pending() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.order_kind = OrderKind::Market;
    terminal.pending_one_shot_status_request = Some(pending_one_shot_status_request());

    let _task = terminal.execute_order(true);

    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(
        status,
        "Wait for pending trading requests to finish before placing an order"
    );
    assert!(terminal.pending_order_action.is_none());
    assert!(terminal.pending_order_indicators.is_empty());
    assert!(terminal.pending_one_shot_status_request.is_some());
}

#[test]
fn execute_order_rejects_while_account_reconciliation_is_pending() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.wallet_address_input = "0xabc0000000000000000000000000000000000000".to_string();
    terminal.set_committed_agent_key_for_test("agent-key");
    terminal.order_kind = OrderKind::Market;
    terminal.order_quantity = "1".to_string();
    terminal.account_reconciliation_required = true;

    let _task = terminal.execute_order(true);

    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(
        status,
        "Account refresh pending; wait for fresh account data before placing an order"
    );
    assert!(terminal.pending_order_action.is_none());
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn execute_order_rejects_blank_connected_address() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("   ".to_string());
    terminal.set_committed_agent_key_for_test("agent-key");
    terminal.order_kind = OrderKind::Market;

    let _task = terminal.execute_order(true);

    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(status, "Connect wallet and enter agent key first");
    assert_eq!(terminal.pending_order_action, None);
}

#[test]
fn execute_order_from_snapshot_rejects_stale_symbol_submit_message() {
    let mut terminal = terminal_for_percentage_order(percentage_account_data());
    terminal.order_kind = OrderKind::Market;
    terminal.order_quantity = "1".to_string();
    terminal.exchange_symbols.push(perp_symbol("ETH"));
    let snapshot = terminal.ticket_order_submission_snapshot();
    terminal.active_symbol = "ETH".to_string();
    terminal.active_symbol_display = "ETH".to_string();

    let _task = terminal.execute_order_from_snapshot(true, snapshot);

    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(status, "Order form changed; review and submit again");
    assert_eq!(terminal.pending_order_action, None);
    assert!(terminal.pending_order_indicators.is_empty());
}

fn perp_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 4,
        max_leverage: 50,
        only_isolated: false,
        market_type: MarketType::Perp,
        outcome: None,
    }
}

fn percentage_account_data() -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "1000".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "1000".to_string(),
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
        fetched_at_ms: TradingTerminal::now_ms(),
    }
}

fn percentage_account_data_with_position(coin: &str, szi: &str) -> AccountData {
    let mut data = percentage_account_data();
    data.clearinghouse.asset_positions = vec![AssetPosition {
        position: Position {
            coin: coin.to_string(),
            szi: szi.to_string(),
            entry_px: "100".to_string(),
            position_value: "0".to_string(),
            unrealized_pnl: "0".to_string(),
            liquidation_px: None,
            leverage: PositionLeverage {
                leverage_type: "cross".to_string(),
                value: 10,
            },
            margin_used: "0".to_string(),
            cum_funding: None,
        },
        liquidation_px: None,
    }];
    data
}

fn terminal_for_percentage_order(account_data: AccountData) -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    let account = "0xabc0000000000000000000000000000000000000";
    terminal.connected_address = Some(account.to_string());
    terminal.wallet_address_input = account.to_string();
    terminal.accounts = vec![AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: account.to_string(),
        agent_key: sensitive_string("").into_zeroizing(),
        hydromancer_api_key: sensitive_string("").into_zeroizing(),
    }];
    terminal.active_account_index = 0;
    terminal.set_committed_agent_key_for_test("agent-key");
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.exchange_symbols = vec![perp_symbol("BTC")];
    terminal.order_kind = OrderKind::Market;
    terminal.order_quantity_is_usd = true;
    terminal.order_reduce_only = false;
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());
    terminal.set_account_data_for_address_for_test(account, account_data);
    terminal
}

#[test]
fn ticket_order_place_intent_matches_alfred_dry_preflight_inputs() {
    let mut terminal = terminal_for_percentage_order(percentage_account_data());
    terminal.order_kind = OrderKind::Limit;
    terminal.order_price = "101.25".to_string();
    terminal.order_quantity = "2.5".to_string();
    terminal.order_quantity_is_usd = false;
    terminal.order_reduce_only = false;

    let order_kind = ExchangeOrderKind::Limit;
    let ticket_intent = TradingTerminal::ticket_order_place_intent(TicketOrderPlaceIntent {
        surface: OrderSurface::Ticket,
        symbol_key: terminal.active_symbol.clone(),
        is_buy: true,
        order_kind,
        price_input: terminal.order_price.clone(),
        quantity_input: terminal.order_quantity.clone(),
        quantity_is_usd: terminal.order_quantity_is_usd,
        reduce_only: terminal.order_reduce_only,
    });
    let alfred_dry_preflight_intent =
        TradingTerminal::ticket_order_place_intent(TicketOrderPlaceIntent {
            surface: OrderSurface::Ticket,
            symbol_key: "BTC".to_string(),
            is_buy: true,
            order_kind,
            price_input: "101.25".to_string(),
            quantity_input: "2.5".to_string(),
            quantity_is_usd: false,
            reduce_only: false,
        });

    let ticket_prepared = terminal
        .prepare_place_order(ticket_intent)
        .expect("ticket order should prepare");
    let alfred_prepared = terminal
        .prepare_place_order(alfred_dry_preflight_intent)
        .expect("alfred dry preflight should prepare");

    assert_eq!(alfred_prepared, ticket_prepared);
    assert_eq!(ticket_prepared.surface, OrderSurface::Ticket);
    assert_eq!(ticket_prepared.symbol_key, "BTC");
    assert_eq!(ticket_prepared.asset, 0);
    assert!(ticket_prepared.is_buy);
    assert_eq!(ticket_prepared.price, "101.25");
    assert_eq!(ticket_prepared.size, "2.5");
    assert_eq!(ticket_prepared.order_kind, ExchangeOrderKind::Limit);
    assert!(!ticket_prepared.reduce_only);
}

#[test]
fn ioc_limit_orders_project_like_market_orders() {
    let (mut terminal, _) = TradingTerminal::boot();
    let account = "0xabc0000000000000000000000000000000000000";
    terminal.connected_address = Some(account.to_string());
    terminal.wallet_address_input = account.to_string();
    terminal.accounts = vec![AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: account.to_string(),
        agent_key: sensitive_string("").into_zeroizing(),
        hydromancer_api_key: sensitive_string("").into_zeroizing(),
    }];
    terminal.active_account_index = 0;
    terminal.set_committed_agent_key_for_test("agent-key");
    terminal.active_symbol = "BTC".to_string();
    terminal.exchange_symbols = vec![perp_symbol("BTC")];
    terminal.order_kind = OrderKind::LimitIoc;
    terminal.order_price = "100".to_string();
    terminal.order_quantity = "1".to_string();
    terminal.order_quantity_is_usd = false;
    terminal.order_reduce_only = false;
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    let symbol = first_symbol_or_panic(&terminal).clone();
    let prepared = prepared_order_or_panic(&terminal, &symbol, true);
    let _task = terminal.submit_prepared_ticket_order(
        "agent-key".to_string().into(),
        account.to_string(),
        prepared,
    );

    // IOC orders are taker orders that never rest: they must project a
    // position delta (MarketPlacing), not a provisional resting row.
    let kinds: Vec<_> = terminal
        .pending_order_indicators
        .values()
        .map(|indicator| indicator.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![crate::order_pending_indicators::PendingOrderIndicatorKind::MarketPlacing]
    );
    let account_addresses: Vec<_> = terminal
        .pending_order_indicators
        .values()
        .map(|indicator| indicator.account_address.as_str())
        .collect();
    assert_eq!(account_addresses, vec![account]);
}

#[test]
fn execute_order_rejects_percentage_quantity_after_account_snapshot_changes() {
    let mut terminal = terminal_for_percentage_order(percentage_account_data());
    terminal.handle_order_percentage_changed(50.0);
    terminal.bump_account_data_revision();

    let _task = terminal.execute_order(true);

    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(status.contains("older account snapshot"));
    assert_eq!(terminal.pending_order_action, None);
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn execute_order_does_not_submit_after_percentage_reselection_loses_account_snapshot() {
    let mut terminal = terminal_for_percentage_order(percentage_account_data());
    terminal.handle_order_percentage_changed(50.0);

    terminal.account_data = None;
    terminal.account_data_address = None;
    terminal.bump_account_data_revision();
    terminal.handle_order_percentage_changed(25.0);
    terminal.set_account_data_for_address_for_test(
        "0xabc0000000000000000000000000000000000000",
        percentage_account_data(),
    );
    let _task = terminal.execute_order(true);

    assert!(terminal.order_quantity.is_empty());
    assert!(terminal.order_quantity_provenance.is_none());
    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(status, "Invalid quantity");
    assert_eq!(terminal.pending_order_action, None);
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn execute_order_rejects_stale_percentage_quantity_snapshot() {
    let mut data = percentage_account_data();
    data.fetched_at_ms = 1;
    let mut terminal = terminal_for_percentage_order(data);
    terminal.handle_order_percentage_changed(50.0);

    let _task = terminal.execute_order(true);

    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(status.contains("Account data is stale for percentage size"));
    assert_eq!(terminal.pending_order_action, None);
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn execute_order_rejects_coin_percentage_quantity_after_reference_price_changes() {
    let mut terminal = terminal_for_percentage_order(percentage_account_data());
    terminal.order_kind = OrderKind::Limit;
    terminal.order_price = "100".to_string();
    terminal.order_quantity_is_usd = false;
    terminal.handle_order_percentage_changed(50.0);
    terminal.handle_order_price_changed("101".to_string());

    let _task = terminal.execute_order(true);

    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(status.contains("reference price changed"));
    assert_eq!(terminal.pending_order_action, None);
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn execute_order_rejects_coin_percentage_quantity_after_order_type_changes() {
    let mut terminal = terminal_for_percentage_order(percentage_account_data());
    terminal.order_kind = OrderKind::Limit;
    terminal.order_price = "100".to_string();
    terminal.order_quantity_is_usd = false;
    terminal.handle_order_percentage_changed(50.0);
    terminal.order_kind = OrderKind::Market;

    let _task = terminal.execute_order(true);

    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(status.contains("order type changed"));
    assert_eq!(terminal.pending_order_action, None);
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn execute_order_allows_usd_percentage_quantity_after_reference_price_changes() {
    let mut terminal = terminal_for_percentage_order(percentage_account_data());
    terminal.order_kind = OrderKind::Limit;
    terminal.order_price = "100".to_string();
    terminal.order_quantity_is_usd = true;
    terminal.handle_order_percentage_changed(50.0);
    terminal.handle_order_price_changed("101".to_string());

    let _task = terminal.execute_order(true);

    assert_eq!(
        terminal.pending_order_action,
        Some(PendingOrderAction::Buy),
        "status: {:?}",
        terminal.order_status
    );
    assert!(!terminal.pending_order_indicators.is_empty());
}

#[test]
fn execute_order_rejects_reduce_only_usd_percentage_quantity_after_reference_price_changes() {
    let mut terminal =
        terminal_for_percentage_order(percentage_account_data_with_position("BTC", "2"));
    terminal.order_kind = OrderKind::Limit;
    terminal.order_price = "100".to_string();
    terminal.order_quantity_is_usd = true;
    terminal.order_reduce_only = true;
    terminal.handle_order_percentage_changed(25.0);
    assert_eq!(terminal.order_quantity, "50.00");
    terminal.handle_order_price_changed("50".to_string());

    let _task = terminal.execute_order(false);

    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(status.contains("reference price changed"));
    assert_eq!(terminal.pending_order_action, None);
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn execute_order_rejects_percentage_quantity_from_incomplete_positions() {
    let mut data = percentage_account_data();
    data.completeness
        .mark_incomplete(AccountDataSection::Positions, "test incomplete positions");
    let mut terminal = terminal_for_percentage_order(data);
    terminal.handle_order_percentage_changed(50.0);

    let _task = terminal.execute_order(true);

    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(status.contains("Positions may be incomplete"));
    assert_eq!(terminal.pending_order_action, None);
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn manual_quantity_after_percentage_size_is_not_tied_to_snapshot() {
    let mut terminal = terminal_for_percentage_order(percentage_account_data());
    terminal.handle_order_percentage_changed(50.0);
    terminal.handle_order_quantity_changed("500".to_string());
    terminal.bump_account_data_revision();

    let _task = terminal.execute_order(true);

    assert_eq!(terminal.pending_order_action, Some(PendingOrderAction::Buy));
    assert!(!terminal.pending_order_indicators.is_empty());
}

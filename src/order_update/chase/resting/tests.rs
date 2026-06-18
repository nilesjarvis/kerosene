use super::{
    chase_resting_order_is_buy, chase_resting_order_wire_is_supported, chase_resting_reduce_only,
};
use crate::account::{
    AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary, OpenOrder,
    SpotClearinghouseState, UserFeeRates,
};
use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::config::AccountProfile;
use crate::order_execution::PendingOrderAction;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

fn account() -> AccountProfile {
    AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: TEST_ACCOUNT.to_string(),
        agent_key: sensitive_string("agent-key").into_zeroizing(),
        hydromancer_api_key: sensitive_string("").into_zeroizing(),
    }
}

fn btc_symbol() -> ExchangeSymbol {
    ExchangeSymbol {
        key: "BTC".to_string(),
        ticker: "BTC".to_string(),
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

fn terminal_with_open_order(order: OpenOrder) -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.accounts = vec![account()];
    terminal.active_account_index = 0;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.wallet_address_input = TEST_ACCOUNT.to_string();
    terminal.set_committed_agent_key_for_test("agent-key");
    terminal.exchange_symbols = vec![btc_symbol()];
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, account_data_with_order(order));
    terminal
}

#[test]
fn resting_chase_preserves_known_perp_reduce_only_metadata() {
    assert_eq!(
        chase_resting_reduce_only(MarketType::Perp, Some(true)),
        Ok(true)
    );
    assert_eq!(
        chase_resting_reduce_only(MarketType::Perp, Some(false)),
        Ok(false)
    );
}

#[test]
fn resting_chase_rejects_unknown_perp_reduce_only_metadata() {
    assert!(
        chase_resting_reduce_only(MarketType::Perp, None)
            .expect_err("unknown reduce-only should be rejected")
            .contains("reduce-only metadata is unavailable")
    );
}

#[test]
fn resting_chase_ignores_spot_reduce_only_metadata() {
    assert_eq!(chase_resting_reduce_only(MarketType::Spot, None), Ok(false));
    assert_eq!(
        chase_resting_reduce_only(MarketType::Spot, Some(true)),
        Ok(false)
    );
}

#[test]
fn resting_chase_side_parser_accepts_only_exchange_sides() {
    assert_eq!(chase_resting_order_is_buy("B"), Some(true));
    assert_eq!(chase_resting_order_is_buy("A"), Some(false));
    assert_eq!(chase_resting_order_is_buy("bad"), None);
}

#[test]
fn resting_chase_rejects_unsupported_wire_order_types() {
    let mut trigger_order = open_order(42);
    trigger_order.is_trigger = Some(true);
    assert!(
        chase_resting_order_wire_is_supported(&trigger_order)
            .expect_err("trigger order should be rejected")
            .contains("trigger orders")
    );

    let mut ioc_order = open_order(42);
    ioc_order.tif = Some("Ioc".to_string());
    assert!(
        chase_resting_order_wire_is_supported(&ioc_order)
            .expect_err("IOC order should be rejected")
            .contains("non-GTC")
    );

    let mut market_order = open_order(42);
    market_order.order_type = Some("Market".to_string());
    assert!(
        chase_resting_order_wire_is_supported(&market_order)
            .expect_err("non-limit order should be rejected")
            .contains("order type")
    );
}

#[test]
fn resting_chase_derives_order_fields_from_current_snapshot() {
    let mut order = open_order(42);
    order.side = "A".to_string();
    order.sz = "0.25".to_string();
    order.limit_px = "101.5".to_string();
    order.reduce_only = Some(true);
    let mut terminal = terminal_with_open_order(order);

    let _task = terminal.handle_chase_resting_order("BTC".to_string(), 42);

    let chase = terminal
        .chase_orders
        .values()
        .next()
        .expect("chase should be adopted");
    assert!(!chase.is_buy);
    assert_eq!(chase.target_size, 0.25);
    assert_eq!(chase.remaining_size, 0.25);
    assert_eq!(chase.current_price, 101.5);
    assert!(chase.reduce_only);
    assert_eq!(chase.current_oid, Some(42));
}

#[test]
fn resting_chase_refuses_account_refresh_in_progress() {
    let mut terminal = terminal_with_open_order(open_order(42));
    terminal.account_loading = true;

    let _task = terminal.handle_chase_resting_order("BTC".to_string(), 42);

    assert!(terminal.chase_orders.is_empty());
    assert_eq!(
        terminal
            .order_status
            .as_ref()
            .map(|(message, is_error)| (message.as_str(), *is_error)),
        Some((
            "Account refresh in progress; wait for fresh open orders before starting chase",
            true
        ))
    );
}

#[test]
fn resting_chase_refuses_stale_snapshot_and_refreshes() {
    let mut terminal = terminal_with_open_order(open_order(42));
    terminal
        .account_data
        .as_mut()
        .expect("account data")
        .fetched_at_ms = 1;

    let _task = terminal.handle_chase_resting_order("BTC".to_string(), 42);

    assert!(terminal.chase_orders.is_empty());
    assert!(terminal.account_loading);
    let (message, is_error) = terminal.order_status.as_ref().expect("order status");
    assert!(*is_error);
    assert!(message.contains("Open orders are stale"));
    assert!(message.contains("refresh before starting chase"));
}

#[test]
fn resting_chase_does_not_treat_positions_refresh_as_open_orders_fresh() {
    let mut terminal = terminal_with_open_order(open_order(42));
    let now_ms = TradingTerminal::now_ms();
    let stale_ms = now_ms.saturating_sub(AccountData::POSITION_ACTION_MAX_AGE_MS + 1_000);
    terminal
        .account_data
        .as_mut()
        .expect("account data")
        .fetched_at_ms = stale_ms;
    terminal
        .account_data
        .as_mut()
        .expect("account data")
        .mark_positions_fetched_at(now_ms);

    let _task = terminal.handle_chase_resting_order("BTC".to_string(), 42);

    assert!(terminal.chase_orders.is_empty());
    assert!(terminal.account_loading);
    let (message, is_error) = terminal.order_status.as_ref().expect("order status");
    assert!(*is_error);
    assert!(message.contains("Open orders are stale"));
    assert!(message.contains("refresh before starting chase"));
}

#[test]
fn resting_chase_refuses_incomplete_open_orders_and_refreshes() {
    let mut terminal = terminal_with_open_order(open_order(42));
    terminal
        .account_data
        .as_mut()
        .expect("account data")
        .completeness
        .open_orders_complete = false;

    let _task = terminal.handle_chase_resting_order("BTC".to_string(), 42);

    assert!(terminal.chase_orders.is_empty());
    assert!(terminal.account_loading);
    assert_eq!(
        terminal
            .order_status
            .as_ref()
            .map(|(message, is_error)| (message.as_str(), *is_error)),
        Some((
            "Open orders are incomplete; refresh before starting chase",
            true
        ))
    );
}

#[test]
fn resting_chase_refuses_same_oid_without_matching_coin() {
    let mut order = open_order(42);
    order.coin = "ETH".to_string();
    let mut terminal = terminal_with_open_order(order);

    let _task = terminal.handle_chase_resting_order("BTC".to_string(), 42);

    assert!(terminal.chase_orders.is_empty());
    assert_eq!(
        terminal
            .order_status
            .as_ref()
            .map(|(message, is_error)| (message.as_str(), *is_error)),
        Some(("Order no longer exists", true))
    );
}

#[test]
fn resting_chase_refuses_active_wallet_mismatch() {
    let mut terminal = terminal_with_open_order(open_order(42));
    terminal.accounts[0].wallet_address = "0xdef0000000000000000000000000000000000000".to_string();

    let _task = terminal.handle_chase_resting_order("BTC".to_string(), 42);

    assert!(terminal.chase_orders.is_empty());
    let (message, is_error) = terminal.order_status.as_ref().expect("order status");
    assert!(*is_error);
    assert!(message.contains("Connected wallet no longer matches the active account"));
}

#[test]
fn resting_chase_refuses_pending_cancel_for_same_oid() {
    let order = open_order(42);
    let mut terminal = terminal_with_open_order(order.clone());
    terminal.add_pending_order_cancellation_indicator(TEST_ACCOUNT.to_string(), &order);

    let _task = terminal.handle_chase_resting_order("BTC".to_string(), 42);

    assert!(terminal.chase_orders.is_empty());
    let (message, is_error) = terminal.order_status.as_ref().expect("order status");
    assert!(*is_error);
    assert!(message.contains("pending cancel"));
    assert!(message.contains("order 42"));
    assert!(message.contains("starting a Chase"));
}

#[test]
fn resting_chase_refuses_pending_order_action() {
    let mut terminal = terminal_with_open_order(open_order(42));
    terminal.pending_order_action = Some(PendingOrderAction::Buy);

    let _task = terminal.handle_chase_resting_order("BTC".to_string(), 42);

    assert!(terminal.chase_orders.is_empty());
    let (message, is_error) = terminal.order_status.as_ref().expect("order status");
    assert!(*is_error);
    assert!(message.contains("pending trading requests"));
    assert!(message.contains("starting a Chase"));
}

use super::{chase, chase_by_id};
use crate::account::{
    AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary, OpenOrder,
    SpotClearinghouseState,
};
use crate::app_state::TradingTerminal;
use crate::order_execution::PendingOrderAction;
use crate::signing::ChaseLifecycle;

fn terminal_ready_for_chase_place() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;
    terminal
}

fn open_order(coin: &str, oid: u64) -> OpenOrder {
    OpenOrder {
        coin: coin.to_string(),
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

fn account_data_with_open_orders(open_orders: Vec<OpenOrder>) -> AccountData {
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
        open_orders,
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: Default::default(),
        completeness: AccountDataCompleteness::default(),
        fetched_at_ms: 1,
    }
}

#[test]
fn chase_place_uses_unfilled_residual_size() {
    let mut terminal = terminal_ready_for_chase_place();
    let mut chase = chase();
    chase.current_oid = None;
    chase.known_oids.clear();
    chase.filled_size = 0.1;
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_place_at_best(1, 101.0);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Placing);
    assert!((chase.remaining_size - 0.9).abs() < 1e-12);
}

#[test]
fn chase_place_ignores_same_oid_open_order_with_mismatched_identity() {
    let mut terminal = terminal_ready_for_chase_place();
    let mut chase = chase();
    chase.current_oid = None;
    chase.known_oids = vec![42];
    terminal.account_data_address = Some(chase.account_address.clone());
    terminal.account_data = Some(account_data_with_open_orders(vec![open_order("ETH", 42)]));
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_place_at_best(1, 101.0);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Placing);
    assert_eq!(chase.current_oid, None);
    assert_eq!(terminal.order_status, None);
}

#[test]
fn chase_replacement_requires_open_order_coverage_for_its_symbol() {
    let mut terminal = terminal_ready_for_chase_place();
    let mut chase = chase();
    chase.coin = "flx:BTC".to_string();
    chase.current_oid = None;
    chase.known_oids = vec![42];
    let mut data = account_data_with_open_orders(Vec::new());
    data.fetch_scope = crate::account::AccountDataFetchScope::hip3_dex("xyz");
    terminal.account_data_address = Some(chase.account_address.clone());
    terminal.account_data = Some(data);
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_place_at_best(1, 101.0);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.current_oid, None);
    assert_eq!(chase.place_attempt_count, 0);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: crate::signing::ChaseVerificationReason::MissingOrder
        }
    );
}

#[test]
fn chase_place_assigns_unique_cloid_per_place_attempt() {
    let mut terminal = terminal_ready_for_chase_place();
    let mut chase = chase();
    chase.current_oid = None;
    chase.known_oids.clear();
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_place_at_best(1, 101.0);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Placing);
    assert_eq!(chase.place_attempt_count, 1);
    assert!(
        chase
            .current_cloid
            .as_deref()
            .is_some_and(|cloid| { cloid.starts_with("0x") && cloid.len() == 34 })
    );
}

#[test]
fn initial_book_load_failure_redacts_provider_error() {
    let mut terminal = terminal_ready_for_chase_place();
    terminal.chase_orders.insert(1, chase());
    terminal.pending_order_action = Some(PendingOrderAction::ChaseBuy);

    let _task = terminal.handle_chase_initial_book_loaded(
        1,
        Err("l2Book request failed: api_key=super-secret".to_string()),
    );

    assert!(terminal.chase_orders.is_empty());
    assert_eq!(terminal.pending_order_action, None);
    let (message, is_error) = terminal.order_status.as_ref().expect("order status");
    assert!(*is_error);
    assert!(message.contains("Chase stopped: book load failed"));
    assert!(message.contains("api_key=<redacted>"));
    assert!(!message.contains("super-secret"));
}

#[test]
fn startup_chase_removal_before_exchange_request_clears_pending_action() {
    let mut terminal = terminal_ready_for_chase_place();
    let mut chase = chase();
    chase.current_oid = None;
    chase.known_oids.clear();
    terminal.chase_orders.insert(1, chase);
    terminal.selected_chase_id = Some(1);
    terminal.pending_order_action = Some(PendingOrderAction::ChaseBuy);

    let _task = terminal.chase_place_at_best(1, f64::NAN);

    assert!(terminal.chase_orders.is_empty());
    assert_eq!(terminal.pending_order_action, None);
}

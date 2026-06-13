use crate::account::{
    AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary,
    SpotClearinghouseState, UserFeeRates,
};
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::config::{self, AccountProfile};
use crate::order_execution::{OneShotPlacementContext, OrderSurface, PendingNukeExecution};
use crate::order_update::PendingOneShotStatusRequest;
use crate::signing::{ChaseLifecycle, ChaseOrder, ExchangeOrderKind};
use crate::twap_state::{TwapOrder, TwapOrderInit, TwapPendingOp, TwapPendingSlice, TwapStatus};

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::time::Duration;
use std::time::Instant;
use zeroize::Zeroize;

fn account(secret_id: &str, name: &str, wallet_address: &str) -> AccountProfile {
    AccountProfile {
        secret_id: secret_id.to_string(),
        name: name.to_string(),
        wallet_address: wallet_address.to_string(),
        agent_key: sensitive_string(format!("{secret_id}-agent-key")).into_zeroizing(),
        hydromancer_api_key: sensitive_string("").into_zeroizing(),
    }
}

fn chase_order(account_address: &str) -> ChaseOrder {
    ChaseOrder {
        id: 42,
        coin: "BTC".to_string(),
        account_address: account_address.to_string(),
        agent_key: sensitive_string("old-account-agent-key")
            .into_zeroizing()
            .into(),
        is_buy: true,
        target_size: 1.0,
        filled_size: 0.0,
        remaining_size: 1.0,
        known_oids: vec![1001],
        current_cloid: None,
        place_attempt_count: 0,
        asset: 0,
        sz_decimals: 5,
        is_spot: false,
        reduce_only: false,
        current_oid: Some(1001),
        current_price: 50_000.0,
        current_price_wire: "50000".to_string(),
        initial_price: 50_000.0,
        started_at: Instant::now(),
        started_at_ms: 1,
        fill_cutoff_ms_by_oid: Vec::new(),
        reprice_count: 0,
        lifecycle: ChaseLifecycle::Resting,
        last_reprice_at: None,
        desired_price: None,
        stop_reason: None,
        cancel_retries: 0,
    }
}

fn pending_one_shot_status_request(account_address: &str) -> PendingOneShotStatusRequest {
    PendingOneShotStatusRequest::new(
        7,
        &OneShotPlacementContext {
            account_address: account_address.to_string(),
            cloid: "0x00000000000000000000000000000000".to_string(),
            surface: OrderSurface::Ticket,
            symbol_key: "BTC".to_string(),
            order_kind: ExchangeOrderKind::Limit,
        },
    )
}

fn empty_account_data() -> AccountData {
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
        fetched_at_ms: TradingTerminal::now_ms(),
    }
}

fn twap_order(id: u64, account_address: &str) -> TwapOrder {
    let now = Instant::now();
    TwapOrder::new(TwapOrderInit {
        id,
        coin: "BTC".to_string(),
        display_coin: "BTC".to_string(),
        account_address: account_address.to_string(),
        agent_key: sensitive_string("old-account-agent-key")
            .into_zeroizing()
            .into(),
        is_buy: true,
        target_size: 1.0,
        asset: 0,
        sz_decimals: 5,
        is_spot: false,
        reduce_only: false,
        min_price: 49_000.0,
        max_price: 51_000.0,
        randomize: false,
        duration: Duration::from_secs(60),
        slice_count: 1,
        now,
        started_at_ms: TradingTerminal::now_ms(),
    })
}

fn pending_place_twap(id: u64, account_address: &str) -> TwapOrder {
    let mut twap = twap_order(id, account_address);
    twap.pending_op = Some(TwapPendingOp::Place(TwapPendingSlice {
        index: 1,
        planned_size: 1.0,
        limit_price: 50_000.0,
        cloid: "0xabc".to_string(),
        retry_count: 0,
    }));
    twap
}

#[test]
fn account_switch_is_blocked_while_chase_order_is_active() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.desktop_notifications = false;
    terminal.accounts = vec![
        account(
            "account-a",
            "Account A",
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        account(
            "account-b",
            "Account B",
            "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ),
    ];
    terminal.active_account_index = 0;
    terminal.wallet_address_input = terminal.accounts[0].wallet_address.clone();
    terminal.wallet_key_input = terminal.accounts[0].agent_key.clone().into();
    terminal.connected_address = Some(terminal.accounts[0].wallet_address.clone());
    terminal.chase_orders.insert(
        42,
        chase_order("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    );

    let _task = terminal.switch_account_task(1);

    assert_eq!(terminal.active_account_index, 0);
    assert_eq!(
        terminal.wallet_address_input,
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert!(terminal.chase_orders.contains_key(&42));
    let toast = terminal.toasts.last().expect("blocked switch should toast");
    assert!(toast.is_error);
    assert!(toast.message.contains("Stop active chase orders"));
}

#[test]
fn account_switch_clears_old_connected_snapshot_before_connect_task() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.accounts = vec![
        account(
            "account-a",
            "Account A",
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        account(
            "account-b",
            "Account B",
            "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ),
    ];
    terminal.active_account_index = 0;
    terminal.wallet_address_input = terminal.accounts[0].wallet_address.clone();
    terminal.wallet_key_input = terminal.accounts[0].agent_key.clone().into();
    terminal.connected_address = Some(terminal.accounts[0].wallet_address.clone());
    terminal.account_data = Some(empty_account_data());
    terminal.account_loading = true;
    terminal.account_reconciliation_required = true;
    terminal.account_error = Some("old account error".to_string());
    terminal.portfolio.last_error = Some("old portfolio error".to_string());
    terminal.income.last_error = Some("old income error".to_string());
    let portfolio_request_id = terminal.portfolio.begin_refresh();
    let income_request_id = terminal.income.begin_refresh();

    let _task = terminal.switch_account_task(1);

    assert_eq!(terminal.active_account_index, 1);
    assert_eq!(
        terminal.wallet_address_input,
        "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    );
    assert_eq!(terminal.wallet_key_input.as_str(), "account-b-agent-key");
    assert_eq!(terminal.connected_address, None);
    assert!(terminal.account_data.is_none());
    assert!(!terminal.account_loading);
    assert!(!terminal.account_reconciliation_required);
    assert_eq!(terminal.account_error, None);
    assert!(!terminal.portfolio.loading);
    assert!(terminal.portfolio.data.is_none());
    assert_eq!(terminal.portfolio.last_error, None);
    assert_ne!(terminal.portfolio.refresh_request_id, portfolio_request_id);
    assert!(!terminal.income.loading);
    assert!(terminal.income.data.is_none());
    assert_eq!(terminal.income.last_error, None);
    assert_ne!(terminal.income.refresh_request_id, income_request_id);
}

#[test]
fn account_switch_is_blocked_while_nuke_execution_is_pending() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.accounts = vec![
        account(
            "account-a",
            "Account A",
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        account(
            "account-b",
            "Account B",
            "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ),
    ];
    terminal.active_account_index = 0;
    terminal.wallet_address_input = terminal.accounts[0].wallet_address.clone();
    terminal.wallet_key_input = terminal.accounts[0].agent_key.clone().into();
    terminal.connected_address = Some(terminal.accounts[0].wallet_address.clone());
    terminal.pending_nuke_execution = Some(PendingNukeExecution::new(7, 2, 0));

    let _task = terminal.switch_account_task(1);

    assert_eq!(terminal.active_account_index, 0);
    assert_eq!(
        terminal.wallet_address_input,
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert!(terminal.pending_nuke_execution.is_some());
    let toast = terminal.toasts.last().expect("blocked switch should toast");
    assert!(toast.is_error);
    assert!(
        toast
            .message
            .contains("pending trading requests to finish before switching accounts")
    );
}

#[test]
fn account_switch_is_blocked_while_one_shot_status_is_pending() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.accounts = vec![
        account(
            "account-a",
            "Account A",
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        account(
            "account-b",
            "Account B",
            "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ),
    ];
    terminal.active_account_index = 0;
    terminal.wallet_address_input = terminal.accounts[0].wallet_address.clone();
    terminal.wallet_key_input = terminal.accounts[0].agent_key.clone().into();
    terminal.connected_address = Some(terminal.accounts[0].wallet_address.clone());
    terminal.pending_one_shot_status_request = Some(pending_one_shot_status_request(
        &terminal.accounts[0].wallet_address,
    ));

    let _task = terminal.switch_account_task(1);

    assert_eq!(terminal.active_account_index, 0);
    assert_eq!(
        terminal.wallet_address_input,
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert!(terminal.pending_one_shot_status_request.is_some());
    let toast = terminal.toasts.last().expect("blocked switch should toast");
    assert!(toast.is_error);
    assert!(
        toast
            .message
            .contains("pending trading requests to finish before switching accounts")
    );
}

#[test]
fn account_switch_stops_old_account_twaps_before_connect_task() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.accounts = vec![
        account(
            "account-a",
            "Account A",
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        account(
            "account-b",
            "Account B",
            "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ),
    ];
    terminal.active_account_index = 0;
    terminal.wallet_address_input = terminal.accounts[0].wallet_address.clone();
    terminal.wallet_key_input = terminal.accounts[0].agent_key.clone().into();
    terminal.connected_address = Some(terminal.accounts[0].wallet_address.clone());
    terminal.twap_orders.insert(
        7,
        twap_order(7, "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    );

    let _task = terminal.switch_account_task(1);

    let twap = terminal.twap_orders.get(&7).expect("twap");
    assert!(twap.stop_requested);
    assert_eq!(twap.status, TwapStatus::Stopped);
    assert_eq!(
        twap.stop_reason
            .as_ref()
            .map(|(reason, is_error)| { (reason.as_str(), *is_error) }),
        Some(("TWAP stopped: account switched", false))
    );
    assert_eq!(terminal.wallet_key_input.as_str(), "account-b-agent-key");
    assert_eq!(terminal.connected_address, None);
}

#[test]
fn account_switch_blocks_twap_with_pending_exchange_state() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.accounts = vec![
        account(
            "account-a",
            "Account A",
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        account(
            "account-b",
            "Account B",
            "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ),
    ];
    terminal.active_account_index = 0;
    terminal.wallet_address_input = terminal.accounts[0].wallet_address.clone();
    terminal.wallet_key_input = terminal.accounts[0].agent_key.clone().into();
    terminal.connected_address = Some(terminal.accounts[0].wallet_address.clone());
    terminal.twap_orders.insert(
        7,
        pending_place_twap(7, "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    );

    let _task = terminal.switch_account_task(1);

    let twap = terminal.twap_orders.get(&7).expect("twap");
    assert!(!twap.stop_requested);
    assert_eq!(terminal.active_account_index, 0);
    assert_eq!(
        terminal.connected_address.as_deref(),
        Some("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
    let toast = terminal.toasts.last().expect("blocked switch should toast");
    assert!(toast.is_error);
    assert!(
        toast
            .message
            .contains("TWAP order status and fill reconciliation")
    );
}

#[test]
fn account_switch_to_same_wallet_different_profile_stops_twap() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.accounts = vec![
        account(
            "account-a",
            "Account A",
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        account(
            "account-b",
            "Account B",
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
    ];
    terminal.active_account_index = 0;
    terminal.wallet_address_input = terminal.accounts[0].wallet_address.clone();
    terminal.wallet_key_input = terminal.accounts[0].agent_key.clone().into();
    terminal.connected_address = Some(terminal.accounts[0].wallet_address.clone());
    terminal.twap_orders.insert(
        7,
        twap_order(7, "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    );
    terminal.account_loading = true;
    let stale_context = terminal.current_account_data_request_context();

    let _task = terminal.switch_account_task(1);

    let twap = terminal.twap_orders.get(&7).expect("twap");
    assert!(twap.stop_requested);
    assert_eq!(twap.status, TwapStatus::Stopped);
    assert_eq!(
        twap.stop_reason
            .as_ref()
            .map(|(reason, is_error)| { (reason.as_str(), *is_error) }),
        Some(("TWAP stopped: account switched", false))
    );
    assert_eq!(terminal.active_account_index, 1);
    assert_eq!(terminal.wallet_key_input.as_str(), "account-b-agent-key");
    assert_eq!(terminal.connected_address, None);
    assert!(!terminal.account_data_request_generation_is_current(
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        stale_context
    ));
}

#[test]
fn account_switch_to_same_wallet_ghost_profile_stops_twap() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.accounts = vec![
        account(
            "account-a",
            "Account A",
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        account(
            "ghost-a",
            "Ghost A",
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
    ];
    terminal.accounts[1].agent_key = sensitive_string("").into_zeroizing();
    terminal.ghost_account_secret_ids = HashSet::from(["ghost-a".to_string()]);
    terminal.active_account_index = 0;
    terminal.wallet_address_input = terminal.accounts[0].wallet_address.clone();
    terminal.wallet_key_input = terminal.accounts[0].agent_key.clone().into();
    terminal.connected_address = Some(terminal.accounts[0].wallet_address.clone());
    terminal.twap_orders.insert(
        7,
        twap_order(7, "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    );

    let _task = terminal.switch_account_task(1);

    let twap = terminal.twap_orders.get(&7).expect("twap");
    assert!(twap.stop_requested);
    assert_eq!(twap.status, TwapStatus::Stopped);
    assert_eq!(
        twap.stop_reason
            .as_ref()
            .map(|(reason, is_error)| { (reason.as_str(), *is_error) }),
        Some(("TWAP stopped: account switched", false))
    );
    assert_eq!(terminal.active_account_index, 1);
    assert!(terminal.wallet_key_input.trim().is_empty());
    assert_eq!(terminal.connected_address, None);
}

#[test]
fn account_switch_does_not_rewrite_terminal_twaps() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.accounts = vec![
        account(
            "account-a",
            "Account A",
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        account(
            "account-b",
            "Account B",
            "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ),
    ];
    terminal.active_account_index = 0;
    terminal.wallet_address_input = terminal.accounts[0].wallet_address.clone();
    terminal.wallet_key_input = terminal.accounts[0].agent_key.clone().into();
    terminal.connected_address = Some(terminal.accounts[0].wallet_address.clone());
    let mut completed = twap_order(9, "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    completed.status = TwapStatus::Completed;
    terminal.twap_orders.insert(9, completed);

    let _task = terminal.switch_account_task(1);

    let twap = terminal.twap_orders.get(&9).expect("twap");
    assert_eq!(twap.status, TwapStatus::Completed);
    assert!(!twap.stop_requested);
    assert_eq!(twap.stop_reason, None);
}

#[test]
fn deferred_legacy_account_key_migrates_profile_hydromancer_key_before_cleanup() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
    terminal.accounts = vec![account(
        "account-a",
        "Account A",
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )];
    terminal.accounts[0].agent_key.zeroize();
    terminal.active_account_index = 0;
    terminal.wallet_key_input.zeroize();
    terminal.hydromancer_api_key.zeroize();
    terminal.hydromancer_key_input.zeroize();
    let saved_payload = RefCell::new(None);

    terminal.load_deferred_legacy_account_key_with(
        0,
        |profile| {
            profile.agent_key = sensitive_string("legacy-agent").into_zeroizing();
            profile.hydromancer_api_key = sensitive_string("legacy-hydro").into_zeroizing();
            Ok(())
        },
        |terminal| {
            saved_payload.replace(Some(config::SecretPayload::from_credentials(
                &terminal.persisted_accounts_snapshot(),
                &terminal.hydromancer_api_key,
                &terminal.hyperdash_api_key,
                &terminal.x_feed.bearer_token,
            )));
            true
        },
    );

    assert_eq!(terminal.wallet_key_input.as_str(), "legacy-agent");
    assert_eq!(terminal.accounts[0].agent_key.as_str(), "legacy-agent");
    assert_eq!(terminal.hydromancer_api_key.as_str(), "legacy-hydro");
    assert_eq!(terminal.hydromancer_key_input.as_str(), "legacy-hydro");
    assert_eq!(terminal.hydromancer_key_generation, 1);
    let payload = saved_payload
        .borrow()
        .clone()
        .expect("migrated credentials should be persisted");
    assert_eq!(payload.profile_agent_key("account-a"), Some("legacy-agent"));
    assert_eq!(payload.global_hydromancer_api_key(), "legacy-hydro");
    let (status, is_error) = terminal
        .secret_store_status
        .as_ref()
        .expect("migration status should be set");
    assert!(!*is_error);
    assert!(status.contains("Legacy account key and Hydromancer key migrated"));
}

#[test]
fn deferred_legacy_account_key_blocks_conflicting_profile_hydromancer_key_cleanup() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
    terminal.accounts = vec![account(
        "account-a",
        "Account A",
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )];
    terminal.accounts[0].agent_key.zeroize();
    terminal.active_account_index = 0;
    terminal.wallet_key_input.zeroize();
    terminal.hydromancer_api_key = sensitive_string("current-hydro").into_zeroizing().into();
    terminal.hydromancer_key_input = terminal.hydromancer_api_key.clone();
    terminal.hydromancer_key_generation = 7;
    let persist_called = Cell::new(false);

    terminal.load_deferred_legacy_account_key_with(
        0,
        |profile| {
            profile.agent_key = sensitive_string("legacy-agent").into_zeroizing();
            profile.hydromancer_api_key = sensitive_string("legacy-hydro").into_zeroizing();
            Ok(())
        },
        |_| {
            persist_called.set(true);
            true
        },
    );

    assert!(!persist_called.get());
    assert!(terminal.wallet_key_input.trim().is_empty());
    assert!(terminal.accounts[0].agent_key.trim().is_empty());
    assert_eq!(terminal.hydromancer_api_key.as_str(), "current-hydro");
    assert_eq!(terminal.hydromancer_key_input.as_str(), "current-hydro");
    assert_eq!(terminal.hydromancer_key_generation, 7);
    let (status, is_error) = terminal
        .secret_store_status
        .as_ref()
        .expect("conflict status should be set");
    assert!(*is_error);
    assert!(status.contains("Multiple legacy Hydromancer API keys"));
    assert!(status.contains("legacy account credentials were left unchanged"));
}

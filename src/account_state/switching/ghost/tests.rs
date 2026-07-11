use super::*;
use crate::signing::{ChaseLifecycle, ChaseOrder};
use crate::twap_state::{TwapOrder, TwapOrderInit, TwapPauseReason};
use crate::wallet_cluster_state::{WalletCluster, WalletClusterMember};

use std::time::{Duration, Instant};
use zeroize::Zeroizing;

const WALLET: &str = "0x1111111111111111111111111111111111111111";
const OTHER_WALLET: &str = "0x2222222222222222222222222222222222222222";

fn account(secret_id: &str, name: &str, wallet_address: &str, agent_key: &str) -> AccountProfile {
    AccountProfile {
        secret_id: secret_id.to_string(),
        name: name.to_string(),
        wallet_address: wallet_address.to_string(),
        agent_key: Zeroizing::new(agent_key.to_string()),
        hydromancer_api_key: Zeroizing::new(String::new()),
    }
}

fn twap_order(id: u64, account_address: &str) -> TwapOrder {
    let now = Instant::now();
    TwapOrder::new(TwapOrderInit {
        id,
        coin: "BTC".to_string(),
        display_coin: "BTC".to_string(),
        account_address: account_address.to_string(),
        agent_key: Zeroizing::new("old-account-agent-key".to_string()).into(),
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

fn status_check_twap(id: u64, account_address: &str) -> TwapOrder {
    let mut twap = twap_order(id, account_address);
    twap.status_check_cloid = Some("0xabc".to_string());
    twap.pause_reason = Some(TwapPauseReason::StatusUnknown);
    twap
}

fn chase_order(account_address: &str) -> ChaseOrder {
    ChaseOrder {
        id: 42,
        coin: "BTC".to_string(),
        account_address: account_address.to_string(),
        agent_key: Zeroizing::new("old-account-agent-key".to_string()).into(),
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

#[test]
fn ghost_wallet_lookup_ignores_saved_trading_profile_with_same_address() {
    let accounts = vec![account("saved", "Saved", WALLET, "agent-key")];
    let ghost_account_secret_ids = HashSet::new();

    assert_eq!(
        find_ghost_account_index(&accounts, &ghost_account_secret_ids, WALLET),
        None
    );
}

#[test]
fn ghost_wallet_lookup_reuses_existing_ghost_profile() {
    let accounts = vec![
        account("saved", "Saved", WALLET, "agent-key"),
        account("ghost", "Ghost", WALLET, ""),
    ];
    let ghost_account_secret_ids = HashSet::from(["ghost".to_string()]);

    assert_eq!(
        find_ghost_account_index(&accounts, &ghost_account_secret_ids, WALLET),
        Some(1)
    );
}

#[test]
fn ghost_wallet_task_does_not_create_profile_when_chase_blocks_new_ghost_switch() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.desktop_notifications = false;
    terminal.accounts = vec![account("saved", "Saved", WALLET, "agent-key")];
    terminal.active_account_index = 0;
    terminal.wallet_address_input = WALLET.to_string();
    terminal.connected_address = Some(WALLET.to_string());
    terminal.last_persisted_active_account_secret_id = Some("saved".to_string());
    terminal.chase_orders.insert(42, chase_order(WALLET));

    let _task = terminal.ghost_wallet_task(OTHER_WALLET.to_string());

    assert_eq!(terminal.accounts.len(), 1);
    assert_eq!(terminal.active_account_index, 0);
    assert_eq!(terminal.wallet_address_input, WALLET);
    assert!(terminal.ghost_account_secret_ids.is_empty());
    assert_eq!(
        terminal.last_persisted_active_account_secret_id.as_deref(),
        Some("saved")
    );
    assert!(terminal.chase_orders.contains_key(&42));
    let toast = terminal
        .toasts
        .last()
        .expect("blocked ghost switch should toast");
    assert!(toast.is_error);
    assert!(toast.message.contains("Stop active chase orders"));
    assert!(toast.message.contains("switching accounts"));
}

#[test]
fn ghost_wallet_task_does_not_create_profile_when_twap_status_is_uncertain() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.desktop_notifications = false;
    terminal.accounts = vec![account("saved", "Saved", WALLET, "agent-key")];
    terminal.active_account_index = 0;
    terminal.wallet_address_input = WALLET.to_string();
    terminal.connected_address = Some(WALLET.to_string());
    terminal.last_persisted_active_account_secret_id = Some("saved".to_string());
    terminal.twap_orders.insert(7, status_check_twap(7, WALLET));

    let _task = terminal.ghost_wallet_task(OTHER_WALLET.to_string());

    assert_eq!(terminal.accounts.len(), 1);
    assert_eq!(terminal.active_account_index, 0);
    assert_eq!(terminal.wallet_address_input, WALLET);
    assert!(terminal.ghost_account_secret_ids.is_empty());
    assert_eq!(
        terminal.last_persisted_active_account_secret_id.as_deref(),
        Some("saved")
    );
    assert!(terminal.twap_orders.contains_key(&7));
    let toast = terminal
        .toasts
        .last()
        .expect("blocked ghost switch should toast");
    assert!(toast.is_error);
    assert!(
        toast
            .message
            .contains("TWAP order status and fill reconciliation")
    );
    assert!(toast.message.contains("switching accounts"));
}

#[test]
fn forget_ghost_wallet_is_blocked_while_twap_order_is_active() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.desktop_notifications = false;
    terminal.accounts = vec![account("ghost", "Ghost", WALLET, "")];
    terminal.ghost_account_secret_ids = HashSet::from(["ghost".to_string()]);
    terminal.active_account_index = 0;
    terminal.wallet_address_input = WALLET.to_string();
    terminal.connected_address = Some(WALLET.to_string());
    terminal.twap_orders.insert(7, twap_order(7, WALLET));

    let _task = terminal.forget_ghost_account_task(0);

    assert_eq!(terminal.accounts.len(), 1);
    assert!(terminal.ghost_account_secret_ids.contains("ghost"));
    assert!(terminal.twap_orders.contains_key(&7));
    let toast = terminal
        .toasts
        .last()
        .expect("blocked ghost forget should toast");
    assert!(toast.is_error);
    assert!(toast.message.contains("Stop active TWAP orders"));
}

#[test]
fn forgetting_selected_cluster_ghost_rotates_cluster_stream_generation() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.accounts = vec![
        account("saved", "Saved", WALLET, "agent-key"),
        account("ghost", "Ghost", OTHER_WALLET, ""),
    ];
    terminal.ghost_account_secret_ids = HashSet::from(["ghost".to_string()]);
    terminal.active_account_index = 0;
    terminal.wallet_clusters.clusters = vec![WalletCluster {
        id: "cluster".to_string(),
        name: "Cluster".to_string(),
        members: vec![WalletClusterMember {
            profile_secret_id: "ghost".to_string(),
            weight: 1.0,
            weight_input: "1".to_string(),
        }],
    }];
    terminal.wallet_clusters.selected_cluster_id = Some("cluster".to_string());
    let previous_generation = terminal.wallet_cluster_user_data_stream_generation;

    let task = terminal.forget_ghost_account_task(1);

    assert_eq!(task.units(), 0);
    assert_eq!(terminal.accounts.len(), 1);
    assert!(terminal.ghost_account_secret_ids.is_empty());
    assert_eq!(
        terminal.wallet_cluster_user_data_stream_generation,
        previous_generation.wrapping_add(1)
    );
}

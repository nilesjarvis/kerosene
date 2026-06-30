use crate::app_state::{TradingTerminal, sensitive_string};
use crate::config::{self, AccountProfile};
use crate::journal::JournalAccountState;
use crate::signing::{ChaseLifecycle, ChaseOrder};
use crate::twap_state::{TwapOrder, TwapOrderInit};

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::time::{Duration, Instant};

mod active_chase;
mod indexes;

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

fn last_toast_or_panic(terminal: &TradingTerminal) -> &crate::notification_state::Toast {
    match terminal.toasts.last() {
        Some(toast) => toast,
        None => panic!("blocked delete should toast"),
    }
}

#[test]
fn active_account_delete_is_blocked_while_twap_order_is_active() {
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
    terminal.accounts[1].agent_key = sensitive_string("").into_zeroizing();
    terminal.active_account_index = 0;
    terminal.wallet_address_input = terminal.accounts[0].wallet_address.clone();
    terminal.wallet_key_input = terminal.accounts[0].agent_key.clone().into();
    terminal.connected_address = Some(terminal.accounts[0].wallet_address.clone());
    terminal.twap_orders.insert(
        7,
        twap_order(7, "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    );

    let _task = terminal.delete_saved_account_task(0);

    assert_eq!(terminal.active_account_index, 0);
    assert_eq!(terminal.accounts.len(), 2);
    assert_eq!(terminal.accounts[0].secret_id, "account-a");
    assert!(terminal.twap_orders.contains_key(&7));
    let toast = last_toast_or_panic(&terminal);
    assert!(toast.is_error);
    assert!(toast.message.contains("Stop active TWAP orders"));
}

#[test]
fn encrypted_account_delete_rewrites_blob_without_deleted_profile_secret() {
    let password = "test-password";
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
    terminal.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
    terminal.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
    terminal.encrypted_secrets_unlocked = true;
    terminal.encrypted_secret_password = sensitive_string(password);
    terminal.hydromancer_api_key = sensitive_string("hydro-key");
    terminal.hyperdash_api_key = sensitive_string("hyper-key");
    terminal.x_feed.set_access_token_from_secret("x-token");

    let _task = terminal.delete_saved_account_task_with_hooks(
        1,
        |terminal, payload| terminal.encrypted_secret_blob_for_payload(payload),
        |_config| Ok(()),
        |_profile| panic!("encrypted config delete should not clear OS keychain profile secrets"),
    );

    assert_eq!(terminal.accounts.len(), 1);
    assert_eq!(terminal.accounts[0].secret_id, "account-a");

    let encrypted = terminal
        .encrypted_secrets
        .as_ref()
        .expect("delete should persist encrypted credentials");
    let payload = config::decrypt_secrets(encrypted, password).expect("payload should decrypt");

    assert_eq!(
        payload.profile_agent_key("account-a"),
        Some("account-a-agent-key")
    );
    assert_eq!(payload.profile_agent_key("account-b"), None);
    assert_eq!(payload.global_hydromancer_api_key(), "hydro-key");
    assert_eq!(payload.global_hyperdash_api_key(), "hyper-key");
    assert_eq!(payload.global_x_access_token(), "x-token");
}

#[test]
fn encrypted_account_delete_save_failure_restores_account_and_original_blob() {
    let password = "test-password";
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
    terminal.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
    terminal.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
    terminal.encrypted_secrets_unlocked = true;
    terminal.encrypted_secret_password = sensitive_string(password);
    terminal.secret_store_status = Some(("Encrypted credentials ready".to_string(), false));
    terminal.secret_migration_save_blocked = true;
    let original_payload = terminal.current_secret_payload();
    let original_encrypted =
        config::encrypt_secrets(&original_payload, password).expect("encrypt fixture");
    terminal.encrypted_secrets = Some(original_encrypted.clone());
    terminal
        .hidden_positions_by_account
        .insert("account-b".to_string(), HashSet::from(["BTC".to_string()]));
    terminal.journal.account_states.insert(
        "account-b".to_string(),
        JournalAccountState {
            loaded_address: Some("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string()),
            ..JournalAccountState::default()
        },
    );
    let save_called = Cell::new(false);
    let keychain_called = Cell::new(false);

    let _task = terminal.delete_saved_account_task_with_hooks(
        1,
        |terminal, payload| terminal.encrypted_secret_blob_for_payload(payload),
        |config| {
            save_called.set(true);
            assert!(
                config
                    .accounts
                    .iter()
                    .all(|profile| profile.secret_id != "account-b")
            );
            let payload = config::decrypt_secrets(
                config
                    .encrypted_secrets
                    .as_ref()
                    .expect("post-delete encrypted blob should be staged before save"),
                password,
            )
            .expect("post-delete encrypted payload should decrypt");
            assert_eq!(payload.profile_agent_key("account-b"), None);
            Err("disk full: api_key=delete-secret".to_string())
        },
        |_profile| {
            keychain_called.set(true);
            Ok(())
        },
    );

    assert!(save_called.get());
    assert!(!keychain_called.get());
    assert_eq!(terminal.accounts.len(), 2);
    assert_eq!(terminal.accounts[1].secret_id, "account-b");
    assert_eq!(
        terminal.encrypted_secrets.as_ref(),
        Some(&original_encrypted)
    );
    assert!(terminal.encrypted_secrets_unlocked);
    assert!(terminal.secret_migration_save_blocked);
    assert_eq!(
        terminal.secret_store_status,
        Some(("Encrypted credentials ready".to_string(), false))
    );
    assert!(
        terminal
            .hidden_positions_by_account
            .contains_key("account-b")
    );
    assert!(terminal.journal.account_states.contains_key("account-b"));
    let payload = config::decrypt_secrets(
        terminal
            .encrypted_secrets
            .as_ref()
            .expect("original encrypted blob remains"),
        password,
    )
    .expect("original payload should decrypt");
    assert_eq!(
        payload.profile_agent_key("account-b"),
        Some("account-b-agent-key")
    );
    let toast = last_toast_or_panic(&terminal);
    assert!(toast.is_error);
    assert!(
        toast
            .message
            .contains("config save failed: disk full: api_key=<redacted>")
    );
    assert!(!toast.message.contains("delete-secret"));
}

#[test]
fn encrypted_account_delete_saves_snapshot_without_deleted_profile_secret() {
    let password = "test-password";
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
    terminal.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
    terminal.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
    terminal.encrypted_secrets_unlocked = true;
    terminal.encrypted_secret_password = sensitive_string(password);
    terminal.hydromancer_api_key = sensitive_string("hydro-key");
    terminal.hyperdash_api_key = sensitive_string("hyper-key");
    let save_called = Cell::new(false);
    let keychain_called = Cell::new(false);

    let _task = terminal.delete_saved_account_task_with_hooks(
        1,
        |terminal, payload| terminal.encrypted_secret_blob_for_payload(payload),
        |config| {
            save_called.set(true);
            assert_eq!(config.accounts.len(), 1);
            assert_eq!(config.accounts[0].secret_id, "account-a");
            let payload = config::decrypt_secrets(
                config
                    .encrypted_secrets
                    .as_ref()
                    .expect("post-delete encrypted blob should be saved"),
                password,
            )
            .expect("post-delete encrypted payload should decrypt");
            assert_eq!(
                payload.profile_agent_key("account-a"),
                Some("account-a-agent-key")
            );
            assert_eq!(payload.profile_agent_key("account-b"), None);
            assert_eq!(payload.global_hydromancer_api_key(), "hydro-key");
            assert_eq!(payload.global_hyperdash_api_key(), "hyper-key");
            Ok(())
        },
        |_profile| {
            keychain_called.set(true);
            Ok(())
        },
    );

    assert!(save_called.get());
    assert!(!keychain_called.get());
    assert_eq!(terminal.accounts.len(), 1);
    assert_eq!(terminal.accounts[0].secret_id, "account-a");
    let toast = last_toast_or_panic(&terminal);
    assert!(!toast.is_error);
    assert!(toast.message.contains("Deleted account: Account B"));
}

#[test]
fn encrypted_active_account_delete_saves_fallback_account_as_active() {
    let password = "test-password";
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
        account(
            "account-c",
            "Account C",
            "0xcccccccccccccccccccccccccccccccccccccccc",
        ),
    ];
    terminal.active_account_index = 1;
    terminal.wallet_address_input = terminal.accounts[1].wallet_address.clone();
    terminal.wallet_key_input = terminal.accounts[1].agent_key.clone().into();
    terminal.last_persisted_active_account_secret_id = Some("account-b".to_string());
    terminal.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
    terminal.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
    terminal.encrypted_secrets_unlocked = true;
    terminal.encrypted_secret_password = sensitive_string(password);
    let save_called = Cell::new(false);

    let _task = terminal.delete_saved_account_task_with_hooks(
        1,
        |terminal, payload| terminal.encrypted_secret_blob_for_payload(payload),
        |config| {
            save_called.set(true);
            assert_eq!(
                config
                    .accounts
                    .iter()
                    .map(|profile| profile.secret_id.as_str())
                    .collect::<Vec<_>>(),
                ["account-a", "account-c"]
            );
            assert_eq!(config.active_account_index, 0);
            let payload = config::decrypt_secrets(
                config
                    .encrypted_secrets
                    .as_ref()
                    .expect("post-delete encrypted blob should be saved"),
                password,
            )
            .expect("post-delete encrypted payload should decrypt");
            assert_eq!(payload.profile_agent_key("account-b"), None);
            Ok(())
        },
        |_profile| panic!("encrypted config delete should not clear OS keychain profile secrets"),
    );

    assert!(save_called.get());
    assert_eq!(terminal.accounts.len(), 2);
    assert_eq!(terminal.active_account_index, 0);
    assert_eq!(terminal.accounts[0].secret_id, "account-a");
    assert_eq!(
        terminal.wallet_address_input,
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
}

#[test]
fn encrypted_account_delete_failure_keeps_account_and_scoped_state() {
    let password = "test-password";
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
    terminal.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
    terminal.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
    terminal.encrypted_secrets_unlocked = true;
    terminal.encrypted_secret_password = sensitive_string(password);
    let original_payload = terminal.current_secret_payload();
    terminal.encrypted_secrets =
        Some(config::encrypt_secrets(&original_payload, password).expect("encrypt fixture"));
    terminal
        .hidden_positions_by_account
        .insert("account-b".to_string(), HashSet::from(["BTC".to_string()]));
    terminal.journal.account_states.insert(
        "account-b".to_string(),
        JournalAccountState {
            loaded_address: Some("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string()),
            ..JournalAccountState::default()
        },
    );

    let _task =
        terminal.delete_saved_account_task_with_encrypted_prepare(1, |_terminal, _payload| None);

    assert_eq!(terminal.accounts.len(), 2);
    assert_eq!(terminal.accounts[1].secret_id, "account-b");
    assert!(
        terminal
            .hidden_positions_by_account
            .contains_key("account-b")
    );
    assert!(terminal.journal.account_states.contains_key("account-b"));
    let encrypted = terminal
        .encrypted_secrets
        .as_ref()
        .expect("original encrypted blob remains");
    let payload = config::decrypt_secrets(encrypted, password).expect("payload should decrypt");
    assert_eq!(
        payload.profile_agent_key("account-b"),
        Some("account-b-agent-key")
    );
    let toast = last_toast_or_panic(&terminal);
    assert!(toast.is_error);
    assert!(
        toast
            .message
            .contains("encrypted credential cleanup failed")
    );
}

#[test]
fn os_keychain_account_delete_save_failure_does_not_clear_keychain() {
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
    terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
    terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
    terminal
        .hidden_positions_by_account
        .insert("account-b".to_string(), HashSet::from(["BTC".to_string()]));
    terminal.journal.account_states.insert(
        "account-b".to_string(),
        JournalAccountState {
            loaded_address: Some("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string()),
            ..JournalAccountState::default()
        },
    );
    let order = RefCell::new(Vec::new());

    let _task = terminal.delete_saved_account_task_with_hooks(
        1,
        |_terminal, _payload| None,
        |config| {
            assert!(order.borrow().is_empty());
            order.borrow_mut().push("save".to_string());
            assert!(
                config
                    .accounts
                    .iter()
                    .all(|profile| profile.secret_id != "account-b")
            );
            assert_eq!(
                config.pending_keychain_profile_deletions.as_slice(),
                ["account-b"]
            );
            Err("disk full: auth_token=delete-secret".to_string())
        },
        |_profile| {
            order.borrow_mut().push("clear-keychain".to_string());
            Ok(())
        },
    );

    assert_eq!(order.borrow().as_slice(), ["save"]);
    assert_eq!(terminal.accounts.len(), 2);
    assert_eq!(terminal.accounts[1].secret_id, "account-b");
    assert!(terminal.pending_keychain_profile_deletions.is_empty());
    assert!(
        terminal
            .hidden_positions_by_account
            .contains_key("account-b")
    );
    assert!(terminal.journal.account_states.contains_key("account-b"));
    let toast = last_toast_or_panic(&terminal);
    assert!(toast.is_error);
    assert!(
        toast
            .message
            .contains("config save failed: disk full: auth_token=<redacted>")
    );
    assert!(!toast.message.contains("delete-secret"));
}

#[test]
fn os_keychain_account_delete_saves_pending_intent_before_keychain_cleanup() {
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
    terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
    terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
    terminal
        .hidden_positions_by_account
        .insert("account-b".to_string(), HashSet::from(["BTC".to_string()]));
    terminal.journal.account_states.insert(
        "account-b".to_string(),
        JournalAccountState {
            loaded_address: Some("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string()),
            ..JournalAccountState::default()
        },
    );
    let order = RefCell::new(Vec::new());

    let _task = terminal.delete_saved_account_task_with_hooks(
        1,
        |_terminal, _payload| None,
        |config| {
            let events = order.borrow().clone();
            match events.as_slice() {
                [] => {
                    order.borrow_mut().push("save-with-intent".to_string());
                    assert_eq!(config.accounts.len(), 1);
                    assert_eq!(config.accounts[0].secret_id, "account-a");
                    assert_eq!(
                        config.pending_keychain_profile_deletions.as_slice(),
                        ["account-b"]
                    );
                    Ok(())
                }
                [first, second] if first == "save-with-intent" && second == "clear-keychain" => {
                    order.borrow_mut().push("save-without-intent".to_string());
                    assert_eq!(config.accounts.len(), 1);
                    assert_eq!(config.accounts[0].secret_id, "account-a");
                    assert!(config.pending_keychain_profile_deletions.is_empty());
                    Ok(())
                }
                other => panic!("unexpected save order: {other:?}"),
            }
        },
        |profile| {
            assert_eq!(order.borrow().as_slice(), ["save-with-intent"]);
            order.borrow_mut().push("clear-keychain".to_string());
            assert_eq!(profile.secret_id, "account-b");
            Ok(())
        },
    );

    assert_eq!(
        order.borrow().as_slice(),
        ["save-with-intent", "clear-keychain", "save-without-intent"]
    );
    assert_eq!(terminal.accounts.len(), 1);
    assert_eq!(terminal.accounts[0].secret_id, "account-a");
    assert!(terminal.pending_keychain_profile_deletions.is_empty());
    assert!(
        !terminal
            .hidden_positions_by_account
            .contains_key("account-b")
    );
    assert!(!terminal.journal.account_states.contains_key("account-b"));
    let toast = last_toast_or_panic(&terminal);
    assert!(!toast.is_error);
    assert!(toast.message.contains("Deleted account: Account B"));
}

#[test]
fn os_keychain_account_delete_cleanup_state_save_failure_redacts_toast() {
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
    terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
    terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
    let order = RefCell::new(Vec::new());

    let _task = terminal.delete_saved_account_task_with_hooks(
        1,
        |_terminal, _payload| None,
        |config| {
            let events = order.borrow().clone();
            match events.as_slice() {
                [] => {
                    order.borrow_mut().push("save-with-intent".to_string());
                    assert_eq!(
                        config.pending_keychain_profile_deletions.as_slice(),
                        ["account-b"]
                    );
                    Ok(())
                }
                [first, second] if first == "save-with-intent" && second == "clear-keychain" => {
                    order.borrow_mut().push("save-without-intent".to_string());
                    assert!(config.pending_keychain_profile_deletions.is_empty());
                    Err("cleanup state failed: signature=cleanup-secret".to_string())
                }
                other => panic!("unexpected save order: {other:?}"),
            }
        },
        |profile| {
            assert_eq!(order.borrow().as_slice(), ["save-with-intent"]);
            order.borrow_mut().push("clear-keychain".to_string());
            assert_eq!(profile.secret_id, "account-b");
            Ok(())
        },
    );

    assert_eq!(
        order.borrow().as_slice(),
        ["save-with-intent", "clear-keychain", "save-without-intent"]
    );
    assert_eq!(terminal.accounts.len(), 1);
    assert_eq!(terminal.accounts[0].secret_id, "account-a");
    let toast = last_toast_or_panic(&terminal);
    assert!(toast.is_error);
    assert!(
        toast
            .message
            .contains("cleanup state save failed: cleanup state failed: signature=<redacted>")
    );
    assert!(!toast.message.contains("cleanup-secret"));
}

#[test]
fn os_keychain_active_account_delete_saves_fallback_account_before_keychain_cleanup() {
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
        account(
            "account-c",
            "Account C",
            "0xcccccccccccccccccccccccccccccccccccccccc",
        ),
    ];
    terminal.active_account_index = 1;
    terminal.wallet_address_input = terminal.accounts[1].wallet_address.clone();
    terminal.wallet_key_input = terminal.accounts[1].agent_key.clone().into();
    terminal.last_persisted_active_account_secret_id = Some("account-b".to_string());
    terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
    terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
    let order = RefCell::new(Vec::new());

    let _task = terminal.delete_saved_account_task_with_hooks(
        1,
        |_terminal, _payload| None,
        |config| {
            let events = order.borrow().clone();
            match events.as_slice() {
                [] => {
                    order.borrow_mut().push("save-with-intent".to_string());
                    assert_eq!(
                        config
                            .accounts
                            .iter()
                            .map(|profile| profile.secret_id.as_str())
                            .collect::<Vec<_>>(),
                        ["account-a", "account-c"]
                    );
                    assert_eq!(config.active_account_index, 0);
                    assert_eq!(
                        config.pending_keychain_profile_deletions.as_slice(),
                        ["account-b"]
                    );
                    Ok(())
                }
                [first, second] if first == "save-with-intent" && second == "clear-keychain" => {
                    order.borrow_mut().push("save-without-intent".to_string());
                    assert_eq!(
                        config
                            .accounts
                            .iter()
                            .map(|profile| profile.secret_id.as_str())
                            .collect::<Vec<_>>(),
                        ["account-a", "account-c"]
                    );
                    assert_eq!(config.active_account_index, 0);
                    assert!(config.pending_keychain_profile_deletions.is_empty());
                    Ok(())
                }
                other => panic!("unexpected save order: {other:?}"),
            }
        },
        |profile| {
            assert_eq!(order.borrow().as_slice(), ["save-with-intent"]);
            order.borrow_mut().push("clear-keychain".to_string());
            assert_eq!(profile.secret_id, "account-b");
            Ok(())
        },
    );

    assert_eq!(
        order.borrow().as_slice(),
        ["save-with-intent", "clear-keychain", "save-without-intent"]
    );
    assert_eq!(terminal.active_account_index, 0);
    assert_eq!(terminal.accounts[0].secret_id, "account-a");
    assert!(terminal.pending_keychain_profile_deletions.is_empty());
    assert_eq!(
        terminal.wallet_address_input,
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
}

#[test]
fn os_keychain_account_delete_cleanup_failure_toast_redacts_account_identifiers() {
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
    terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
    terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
    let saved_snapshots = RefCell::new(Vec::new());
    let order = RefCell::new(Vec::new());

    let _task = terminal.delete_saved_account_task_with_hooks(
        1,
        |_terminal, _payload| None,
        |config| {
            assert!(order.borrow().is_empty());
            order.borrow_mut().push("save-with-intent".to_string());
            assert_eq!(
                config.pending_keychain_profile_deletions.as_slice(),
                ["account-b"]
            );
            saved_snapshots.borrow_mut().push(
                config
                    .accounts
                    .iter()
                    .map(|profile| profile.secret_id.clone())
                    .collect::<Vec<_>>(),
            );
            Ok(())
        },
        |_profile| {
            assert_eq!(order.borrow().as_slice(), ["save-with-intent"]);
            order.borrow_mut().push("clear-keychain".to_string());
            Err("account-b cleanup failed for Account B".to_string())
        },
    );

    assert_eq!(
        order.borrow().as_slice(),
        ["save-with-intent", "clear-keychain"]
    );
    assert_eq!(
        saved_snapshots.borrow().as_slice(),
        &[vec!["account-a".to_string()]]
    );
    assert_eq!(terminal.accounts.len(), 1);
    assert_eq!(terminal.accounts[0].secret_id, "account-a");
    assert_eq!(
        terminal.pending_keychain_profile_deletions.as_slice(),
        ["account-b"]
    );
    let toast = last_toast_or_panic(&terminal);
    assert!(toast.is_error);
    assert_eq!(
        toast.message,
        "Deleted account, but OS keychain cleanup failed and will retry"
    );
    assert!(!toast.message.contains("account-b"));
    assert!(!toast.message.contains("Account B"));
}

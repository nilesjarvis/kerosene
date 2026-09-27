use super::*;
use crate::config::{AddressBookEntryConfig, KeroseneConfig, RemoteWalletDatabaseConfig};
use crate::wallet_state::AddressBookEntry;

const LOCAL: &str = "0x1111111111111111111111111111111111111111";
const REMOTE: &str = "0x2222222222222222222222222222222222222222";
const OTHER: &str = "0x3333333333333333333333333333333333333333";

fn terminal() -> TradingTerminal {
    TradingTerminal::boot_from_config(KeroseneConfig {
        remote_wallet_database: RemoteWalletDatabaseConfig {
            url: "http://wallets.test".into(),
        },
        address_book: vec![AddressBookEntryConfig {
            address: LOCAL.into(),
            label: "Local label".into(),
            ..Default::default()
        }],
        ..Default::default()
    })
    .0
}

fn snapshot(entries: &[(&str, &str)]) -> RemoteWalletSnapshot {
    RemoteWalletSnapshot {
        entries: entries
            .iter()
            .map(|(address, label)| {
                (
                    address.to_string(),
                    AddressBookEntry {
                        label: label.to_string(),
                        ..Default::default()
                    },
                )
            })
            .collect(),
        skipped: 0,
    }
}

fn complete(terminal: &mut TradingTerminal, entries: &[(&str, &str)]) {
    let _ = terminal.request_remote_wallet_sync();
    let request_id = terminal
        .wallet_tracker
        .remote_database
        .pending_request
        .expect("pending sync");
    let _ = terminal.update(Message::RemoteWalletDatabaseLoaded(
        request_id,
        RemoteWalletDatabaseResult(Ok(snapshot(entries))),
    ));
}

fn saved_config(terminal: &mut TradingTerminal) -> KeroseneConfig {
    let mut saved = None;
    terminal
        .persist_config_immediately_with(|config| {
            saved = Some(config.clone());
            Ok(())
        })
        .expect("snapshot");
    saved.expect("saved config")
}

#[test]
fn startup_sync_reconciles_additions_renames_deletions_and_local_overlap() {
    let mut terminal = terminal();
    assert!(
        terminal
            .wallet_tracker
            .remote_database
            .pending_request
            .is_some()
    );
    complete(
        &mut terminal,
        &[(LOCAL, "Remote override"), (REMOTE, "First name")],
    );
    assert_eq!(terminal.wallet_tracker.tracked_addresses, [LOCAL, REMOTE]);
    assert_eq!(terminal.wallet_label(LOCAL), Some("Remote override"));
    assert_eq!(terminal.wallet_display(REMOTE).primary, "First name");
    assert_eq!(
        terminal.tracked_trade_subscription_addresses(),
        [LOCAL, REMOTE]
    );
    let nonce = terminal.tracked_trades_reconnect_nonce;
    complete(
        &mut terminal,
        &[(LOCAL, "Remote override"), (REMOTE, "Renamed")],
    );
    assert_eq!(terminal.wallet_label(REMOTE), Some("Renamed"));
    assert_eq!(terminal.tracked_trades_reconnect_nonce, nonce);
    complete(&mut terminal, &[(OTHER, "Replacement")]);
    assert_eq!(terminal.wallet_tracker.tracked_addresses, [LOCAL, OTHER]);
    assert_eq!(terminal.wallet_label(LOCAL), Some("Local label"));
    assert!(terminal.wallet_label(REMOTE).is_none());
    assert!(!terminal.wallet_tracker.rows.contains_key(REMOTE));
    assert!(
        !terminal
            .wallet_tracker
            .core_refresh_queue
            .iter()
            .any(|address| address == REMOTE)
    );
    assert_eq!(
        terminal.tracked_trade_subscription_addresses(),
        [LOCAL, OTHER]
    );
    complete(&mut terminal, &[]);
    assert_eq!(terminal.wallet_tracker.tracked_addresses, [LOCAL]);
}

#[test]
fn remote_records_and_mutes_never_enter_saved_config_or_exports() {
    let mut terminal = terminal();
    complete(
        &mut terminal,
        &[(LOCAL, "Remote override"), (REMOTE, "Remote-only label")],
    );
    let _ = terminal.update(Message::WalletTrackerMute(REMOTE.into()));
    let saved = saved_config(&mut terminal);
    assert_eq!(saved.remote_wallet_database.url, "http://wallets.test");
    assert_eq!(saved.wallet_tracker.tracked_addresses, [LOCAL]);
    assert_eq!(saved.wallet_tracker.wallets[0].label, "Local label");
    assert!(saved.wallet_tracker.muted_addresses.is_empty());
    let json = serde_json::to_string(&saved).expect("JSON");
    for private in [REMOTE, "Remote override", "Remote-only label"] {
        assert!(!json.contains(private));
        assert!(
            !serde_json::to_string(&terminal.wallet_labels_export_with_time(0))
                .expect("export")
                .contains(private)
        );
    }
    let rebooted = TradingTerminal::boot_from_config(saved).0;
    assert!(rebooted.wallet_tracker.remote_database.entries.is_empty());
    assert_eq!(rebooted.wallet_tracker.tracked_addresses, [LOCAL]);
    assert!(
        rebooted
            .wallet_tracker
            .remote_database
            .pending_request
            .is_some()
    );
}

#[test]
fn remote_entries_cannot_be_renamed_deleted_or_relabelled_by_unmuting() {
    let mut terminal = terminal();
    complete(&mut terminal, &[(REMOTE, "Authoritative")]);
    let _ = terminal.update(Message::WalletTrackerLabelChanged(
        REMOTE.into(),
        "Override".into(),
    ));
    let _ = terminal.update(Message::WalletTrackerRemove(REMOTE.into()));
    let _ = terminal.update(Message::WalletTrackerMute(REMOTE.into()));
    let _ = terminal.update(Message::WalletTrackerInputChanged(REMOTE.into()));
    let _ = terminal.update(Message::WalletTrackerLabelInputChanged("Override".into()));
    let _ = terminal.update(Message::WalletTrackerAdd);
    assert!(
        terminal
            .wallet_tracker
            .tracked_addresses
            .iter()
            .any(|address| address == REMOTE)
    );
    assert_eq!(terminal.wallet_label(REMOTE), Some("Authoritative"));
    assert!(!terminal.address_book.contains_key(REMOTE));
    assert!(!terminal.wallet_tracker.is_muted(REMOTE));
}

#[test]
fn failures_keep_memory_and_switch_and_disconnect_ignore_stale_results() {
    let mut terminal = terminal();
    complete(&mut terminal, &[(REMOTE, "Retained")]);
    let _ = terminal.request_remote_wallet_sync();
    let request = terminal
        .wallet_tracker
        .remote_database
        .pending_request
        .expect("request");
    let _ = terminal.request_remote_wallet_sync();
    assert_eq!(
        terminal.wallet_tracker.remote_database.pending_request,
        Some(request)
    );
    let _ = terminal.update(Message::RemoteWalletDatabaseLoaded(
        request,
        RemoteWalletDatabaseResult(Err("Unavailable".into())),
    ));
    assert_eq!(terminal.wallet_label(REMOTE), Some("Retained"));
    assert!(
        terminal
            .wallet_tracker
            .remote_database
            .status_text()
            .contains("showing last sync")
    );
    let _ = terminal.request_remote_wallet_sync();
    let stale = terminal
        .wallet_tracker
        .remote_database
        .pending_request
        .expect("old request");
    let _ = terminal.update(Message::RemoteWalletDatabaseUrlChanged(
        "http://other.test/".into(),
    ));
    let _ = terminal.update(Message::SaveRemoteWalletDatabase);
    let new_request = terminal.wallet_tracker.remote_database.pending_request;
    assert_ne!(new_request, Some(stale));
    assert!(
        !terminal
            .wallet_tracker
            .tracked_addresses
            .iter()
            .any(|address| address == REMOTE)
    );
    let _ = terminal.update(Message::RemoteWalletDatabaseLoaded(
        stale,
        RemoteWalletDatabaseResult(Ok(snapshot(&[(REMOTE, "Stale")]))),
    ));
    assert_eq!(
        terminal.wallet_tracker.remote_database.pending_request,
        new_request
    );
    assert!(terminal.wallet_tracker.remote_database.entries.is_empty());
    complete(&mut terminal, &[(OTHER, "New source")]);
    let _ = terminal.update(Message::DisconnectRemoteWalletDatabase);
    assert!(terminal.wallet_tracker.remote_database.url.is_empty());
    assert_eq!(terminal.wallet_tracker.tracked_addresses, [LOCAL]);
    assert!(
        saved_config(&mut terminal)
            .remote_wallet_database
            .url
            .is_empty()
    );
}

#[test]
fn invalid_input_keeps_active_connection_and_does_not_leak_credentials() {
    let mut terminal = terminal();
    let _ = terminal.update(Message::RemoteWalletDatabaseUrlChanged(
        "https://user:private-sentinel@host".into(),
    ));
    let _ = terminal.update(Message::SaveRemoteWalletDatabase);
    assert_eq!(
        terminal.wallet_tracker.remote_database.url,
        "http://wallets.test"
    );
    assert!(
        terminal
            .wallet_tracker
            .remote_database
            .input_error
            .is_some()
    );
    assert!(
        !format!(
            "{:?}",
            Message::RemoteWalletDatabaseUrlChanged("private-sentinel".into())
        )
        .contains("private-sentinel")
    );
    assert!(
        !format!(
            "{:?}",
            RemoteWalletDatabaseResult(Ok(snapshot(&[(REMOTE, "private-sentinel")])))
        )
        .contains("private-sentinel")
    );
    assert!(
        !format!(
            "{:?}",
            RemoteWalletDatabaseResult(Err("private-sentinel".into()))
        )
        .contains("private-sentinel")
    );
}

#[test]
fn explicitly_imported_local_label_survives_remote_removal() {
    let mut terminal = terminal();
    complete(&mut terminal, &[(REMOTE, "Remote label")]);
    terminal.address_book.insert(
        REMOTE.into(),
        AddressBookEntry {
            label: "Imported local label".into(),
            ..Default::default()
        },
    );
    assert!(
        saved_config(&mut terminal)
            .wallet_tracker
            .tracked_addresses
            .iter()
            .any(|address| address == REMOTE)
    );
    complete(&mut terminal, &[]);
    assert!(
        terminal
            .wallet_tracker
            .tracked_addresses
            .iter()
            .any(|address| address == REMOTE)
    );
    assert_eq!(terminal.wallet_label(REMOTE), Some("Imported local label"));
}

#[tokio::test]
async fn config_clear_invalidates_pending_sync_and_removes_runtime_records() {
    let _cursor_guard = crate::telegram_fast_feed::fast_channel_cursor_test_lock()
        .lock()
        .await;
    let mut terminal = terminal();
    complete(&mut terminal, &[(REMOTE, "Remote")]);
    let _ = terminal.request_remote_wallet_sync();
    let stale = terminal
        .wallet_tracker
        .remote_database
        .pending_request
        .expect("pending");
    let _ = terminal.apply_config_clear_to_runtime(crate::config::ClearConfigSummary {
        files_removed: 0,
        file_cleanup_failed: false,
        keychain_entries_cleared: 0,
        warnings: Vec::new(),
    });
    let _ = terminal.update(Message::RemoteWalletDatabaseLoaded(
        stale,
        RemoteWalletDatabaseResult(Ok(snapshot(&[(REMOTE, "Stale")]))),
    ));
    assert!(terminal.wallet_tracker.remote_database.url.is_empty());
    assert!(terminal.wallet_tracker.remote_database.entries.is_empty());
    assert!(
        terminal
            .wallet_tracker
            .remote_database
            .pending_request
            .is_none()
    );
    assert!(terminal.wallet_tracker.tracked_addresses.is_empty());
}

#[test]
fn invalid_saved_url_does_not_schedule_a_request_or_reset_other_settings() {
    let terminal = TradingTerminal::boot_from_config(KeroseneConfig {
        remote_wallet_database: RemoteWalletDatabaseConfig {
            url: "not a URL".into(),
        },
        ..Default::default()
    })
    .0;
    assert!(terminal.wallet_tracker.remote_database.url.is_empty());
    assert!(
        terminal
            .wallet_tracker
            .remote_database
            .input_error
            .is_some()
    );
    assert!(
        terminal
            .wallet_tracker
            .remote_database
            .pending_request
            .is_none()
    );
}

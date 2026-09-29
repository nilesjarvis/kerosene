use super::*;
use crate::config::KeroseneConfig;
use crate::wallet_state::AddressBookEntry;

const ADDRESS: &str = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const BLOCKER: &str = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn terminal(restoring: bool, label: &str) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    if restoring {
        terminal
            .wallet_tracker
            .tracked_addresses
            .push(ADDRESS.into());
    }
    terminal.wallet_tracker.muted_addresses.push(ADDRESS.into());
    terminal.wallet_tracker.add_input = format!(" {} ", ADDRESS.to_uppercase());
    terminal.wallet_tracker.add_label_input = label.into();
    terminal.config_save_due_at = None;
    terminal
}

#[test]
fn wallet_add_shares_completion_but_preserves_new_and_restored_row_policies() {
    for restoring in [false, true] {
        for label in [" Revised ", " \t"] {
            let mut terminal = terminal(restoring, label);
            terminal.address_book.insert(
                ADDRESS.into(),
                AddressBookEntry {
                    label: "Existing".into(),
                    color: Some("#ff00ff".into()),
                    tags: vec!["desk".into()],
                },
            );
            terminal.wallet_tracker.rows.insert(
                ADDRESS.into(),
                WalletTrackerRow {
                    order_error: Some("previous order error".into()),
                    last_updated_ms: Some(42),
                    ..Default::default()
                },
            );
            terminal.wallet_tracker.rows.insert(
                BLOCKER.into(),
                WalletTrackerRow {
                    loading: true,
                    ..Default::default()
                },
            );
            let nonce = terminal.tracked_trades_reconnect_nonce;
            let _task = terminal.update_wallet_tracker_entries(Message::WalletTrackerAdd);

            assert_eq!(terminal.wallet_tracker.tracked_addresses, [ADDRESS]);
            assert!(terminal.wallet_tracker.muted_addresses.is_empty());
            assert!(terminal.wallet_tracker.add_input.is_empty());
            assert!(terminal.wallet_tracker.add_label_input.is_empty());
            assert!(terminal.config_save_due_at.is_some());
            assert_eq!(terminal.wallet_tracker.core_refresh_queue, [ADDRESS]);
            assert_eq!(
                terminal.tracked_trades_reconnect_nonce,
                nonce.wrapping_add(u64::from(restoring || !label.trim().is_empty()))
            );
            let entry = &terminal.address_book[ADDRESS];
            assert_eq!(
                entry.label,
                if label.trim().is_empty() {
                    "Existing"
                } else {
                    "Revised"
                }
            );
            assert_eq!(entry.color.as_deref(), Some("#ff00ff"));
            assert_eq!(entry.tags, ["desk"]);
            let row = &terminal.wallet_tracker.rows[ADDRESS];
            assert!(!row.loading);
            assert_eq!(
                row.order_error.as_deref(),
                restoring.then_some("previous order error")
            );
            assert_eq!(row.last_updated_ms, restoring.then_some(42));
        }
    }
}

#[test]
fn wallet_add_and_restore_start_refresh_after_saving_and_clearing_inputs() {
    for restoring in [false, true] {
        let mut terminal = terminal(restoring, "Desk");
        let context = terminal.read_data_request_context();
        let _task = terminal.update_wallet_tracker_entries(Message::WalletTrackerAdd);
        assert!(terminal.config_save_due_at.is_some());
        assert!(terminal.wallet_tracker.add_input.is_empty());
        assert!(terminal.wallet_tracker.add_label_input.is_empty());
        assert!(terminal.wallet_tracker.core_refresh_queue.is_empty());
        assert_eq!(terminal.wallet_tracker.tracked_addresses, [ADDRESS]);
        assert!(!terminal.wallet_tracker.is_muted(ADDRESS));
        let row = &terminal.wallet_tracker.rows[ADDRESS];
        assert!(row.loading);
        assert_eq!(row.loading_context, Some(context));
    }
}

#[test]
fn rejected_wallet_add_preserves_inputs_and_does_not_save_or_refresh() {
    for (input, expected) in [
        ("invalid", "Invalid wallet address"),
        (ADDRESS, "Wallet already shown in tracker"),
    ] {
        let mut terminal = terminal(true, "Pending label");
        terminal.wallet_tracker.muted_addresses.clear();
        terminal.wallet_tracker.add_input = input.into();
        terminal.toasts.clear();
        let nonce = terminal.tracked_trades_reconnect_nonce;
        let _task = terminal.update_wallet_tracker_entries(Message::WalletTrackerAdd);
        assert_eq!(terminal.wallet_tracker.add_input, input);
        assert_eq!(terminal.wallet_tracker.add_label_input, "Pending label");
        assert!(terminal.config_save_due_at.is_none());
        assert_eq!(terminal.tracked_trades_reconnect_nonce, nonce);
        assert_eq!(terminal.wallet_tracker.tracked_addresses, [ADDRESS]);
        assert!(terminal.wallet_tracker.rows.is_empty());
        assert!(terminal.wallet_tracker.core_refresh_queue.is_empty());
        assert!(terminal.address_book.is_empty());
        assert_eq!(terminal.toasts.len(), 1);
        assert_eq!(terminal.toasts[0].message, expected);
        assert!(terminal.toasts[0].is_error);
    }
}

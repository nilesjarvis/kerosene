use super::*;
use crate::api::UserFill;

fn fill(time: u64, tid: u64, coin: &str) -> UserFill {
    UserFill {
        coin: coin.to_string(),
        px: "100.0".to_string(),
        sz: "1.0".to_string(),
        side: "B".to_string(),
        time,
        start_position: "0.0".to_string(),
        dir: "Open Long".to_string(),
        closed_pnl: "0.0".to_string(),
        hash: format!("0x{time:x}{tid:x}"),
        oid: tid + 100,
        crossed: false,
        fee: "0.01".to_string(),
        tid,
        fee_token: "USDC".to_string(),
    }
}

fn note(open: &str) -> JournalNote {
    JournalNote {
        open: open.to_string(),
        close: String::new(),
    }
}

#[test]
fn normalize_fills_sorts_and_deduplicates_by_composite_identity() {
    let duplicate = fill(3, 30, "ETH");
    let mut fills = vec![
        duplicate.clone(),
        fill(1, 10, "BTC"),
        duplicate,
        fill(2, 20, "SOL"),
    ];

    normalize_fills(&mut fills);

    assert_eq!(fills.len(), 3);
    assert_eq!(fills[0].time, 1);
    assert_eq!(fills[1].time, 2);
    assert_eq!(fills[2].time, 3);
}

#[test]
fn merge_fills_uses_composite_identity_not_tid_only() {
    let mut existing = vec![fill(1, 10, "BTC")];
    let mut same_tid_different_fill = fill(2, 10, "ETH");
    same_tid_different_fill.hash = "0xdifferent".to_string();

    let added = merge_fills(
        &mut existing,
        vec![fill(1, 10, "BTC"), same_tid_different_fill],
    );

    assert_eq!(added, 1);
    assert_eq!(existing.len(), 2);
    assert_eq!(newest_fill_time(&existing), Some(2));
}

#[test]
fn aggregate_trades_skips_malformed_numeric_fills() {
    let mut malformed = fill(1, 10, "BTC");
    malformed.sz = "not-a-number".to_string();

    let result = aggregate_trades_with_diagnostics(vec![malformed]);

    assert!(result.trades.is_empty());
    assert_eq!(result.diagnostics.skipped_fill_count, 1);
}

#[test]
fn aggregate_trades_marks_missing_opening_basis_as_partial() {
    let mut close = fill(1, 10, "BTC");
    close.side = "A".to_string();
    close.start_position = "1.0".to_string();
    close.dir = "Close Long".to_string();
    close.closed_pnl = "10.0".to_string();

    let result = aggregate_trades_with_diagnostics(vec![close]);

    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.diagnostics.incomplete_trade_count, 1);
    assert!(!result.trades[0].basis_complete);
    assert_eq!(result.trades[0].pnl, 10.0);
}

#[test]
fn note_lookup_keeps_legacy_time_based_keys_working() {
    let result = aggregate_trades_with_diagnostics(vec![fill(1, 10, "BTC")]);
    let trade = &result.trades[0];
    let legacy_key = "BTC_1".to_string();
    let mut entries = HashMap::new();
    entries.insert(
        legacy_key.clone(),
        JournalNote {
            open: "legacy note".to_string(),
            close: String::new(),
        },
    );

    assert_ne!(trade.id, legacy_key);
    assert_eq!(note_key_for_trade(&entries, trade), Some(legacy_key));
    assert_eq!(
        note_for_trade(&entries, trade).map(|note| note.open.as_str()),
        Some("legacy note")
    );
}

#[test]
fn journal_state_migrates_legacy_notes_to_active_account() {
    let mut legacy_entries = HashMap::new();
    legacy_entries.insert("BTC_1".to_string(), note("legacy"));

    let state = JournalState::new_for_account(
        Some("account-a".to_string()),
        HashMap::new(),
        legacy_entries,
    );

    assert_eq!(
        state.entries.get("BTC_1").map(|entry| entry.open.as_str()),
        Some("legacy")
    );
    assert_eq!(
        state
            .account_states
            .get("account-a")
            .and_then(|account| account.entries.get("BTC_1"))
            .map(|entry| entry.open.as_str()),
        Some("legacy")
    );
}

#[test]
fn journal_state_switches_entries_by_account() {
    let mut account_entries = HashMap::new();
    account_entries.insert(
        "account-a".to_string(),
        HashMap::from([("a".to_string(), note("a"))]),
    );
    account_entries.insert(
        "account-b".to_string(),
        HashMap::from([("b".to_string(), note("b"))]),
    );

    let mut state = JournalState::new_for_account(
        Some("account-a".to_string()),
        account_entries,
        HashMap::new(),
    );
    state.entries.insert("a2".to_string(), note("a2"));

    state.switch_active_account(Some("account-b".to_string()));
    assert!(state.entries.contains_key("b"));
    assert!(!state.entries.contains_key("a"));
    state.entries.insert("b2".to_string(), note("b2"));

    state.switch_active_account(Some("account-a".to_string()));
    assert!(state.entries.contains_key("a"));
    assert!(state.entries.contains_key("a2"));
    assert!(!state.entries.contains_key("b2"));

    state.switch_active_account(Some("account-b".to_string()));
    assert!(state.entries.contains_key("b2"));
}

#[test]
fn journal_entries_snapshot_includes_current_active_entries() {
    let mut state = JournalState::new_for_account(
        Some("account-a".to_string()),
        HashMap::new(),
        HashMap::new(),
    );
    state.entries.insert("active".to_string(), note("active"));

    let snapshot = state.entries_by_account_snapshot();

    assert_eq!(
        snapshot
            .get("account-a")
            .and_then(|entries| entries.get("active"))
            .map(|entry| entry.open.as_str()),
        Some("active")
    );
}

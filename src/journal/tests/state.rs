use super::note;
use crate::journal::JournalState;
use std::collections::HashMap;

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

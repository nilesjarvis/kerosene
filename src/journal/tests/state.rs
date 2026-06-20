use super::note;
use crate::{
    config::ChartBackfillSource,
    journal::{
        JournalAccountState, JournalFilter, JournalState, JournalSyncStatus,
        JournalTradeSnapshotRequest,
    },
    timeframe::Timeframe,
};
use std::collections::HashMap;

#[test]
fn journal_filter_matches_expected_coin_prefixes() {
    let cases = [
        ("BTC", true, true, false, false),
        ("xyz:NVDA", true, true, false, false),
        ("@107", true, false, true, false),
        ("#950", true, false, false, true),
    ];

    for (coin, all, perp, spot, outcome) in cases {
        assert_eq!(JournalFilter::All.matches_coin(coin), all, "{coin} all");
        assert_eq!(JournalFilter::Perp.matches_coin(coin), perp, "{coin} perp");
        assert_eq!(JournalFilter::Spot.matches_coin(coin), spot, "{coin} spot");
        assert_eq!(
            JournalFilter::Outcome.matches_coin(coin),
            outcome,
            "{coin} outcome"
        );
    }
}

#[test]
fn journal_account_state_debug_redacts_account_scoped_data() {
    const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";
    const ACCOUNT_KEY: &str = "journal-account-key-sentinel";
    const NOTE_TEXT: &str = "private journal note sentinel";
    const DRAFT_TEXT: &str = "private draft sentinel";

    let mut state = JournalAccountState {
        loaded_address: Some(ADDRESS.to_string()),
        ..JournalAccountState::default()
    };
    state.entries.insert("trade-1".to_string(), note(NOTE_TEXT));
    state
        .edit_buffers
        .insert("trade-1".to_string(), note(DRAFT_TEXT));
    state.snapshot_requests.insert(
        "trade-1".to_string(),
        JournalTradeSnapshotRequest {
            account_key: Some(ACCOUNT_KEY.to_string()),
            address: ADDRESS.to_string(),
            trade_id: "trade-1".to_string(),
            coin: "HYPE".to_string(),
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: 1,
            hydromancer_key_generation: 2,
            coverage: crate::journal::JournalSnapshotCoverage::default(),
            timeframe: Timeframe::M1,
            ladder_index: 0,
            trade_start_ms: 100,
            trade_end_ms: 200,
            is_open: false,
            start_ms: 50,
            end_ms: 250,
        },
    );
    state.error = Some(format!("failed for {ADDRESS} api_key=journal-secret"));
    state.warning = Some(format!("warning for {ADDRESS}"));

    let rendered = format!("{state:?}");

    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains(ADDRESS), "{rendered}");
    assert!(!rendered.contains(ACCOUNT_KEY), "{rendered}");
    assert!(!rendered.contains("journal-secret"), "{rendered}");
    assert!(!rendered.contains(NOTE_TEXT), "{rendered}");
    assert!(!rendered.contains(DRAFT_TEXT), "{rendered}");
    assert!(rendered.contains("entries: len=1"), "{rendered}");
    assert!(rendered.contains("snapshot_requests: len=1"), "{rendered}");
    assert!(rendered.contains("edit_buffers: len=1"), "{rendered}");
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

#[test]
fn journal_snapshot_expansion_is_scoped_by_account() {
    let mut state = JournalState::new_for_account(
        Some("account-a".to_string()),
        HashMap::new(),
        HashMap::new(),
    );

    state
        .expanded_snapshot_trade_ids
        .insert("trade-a".to_string());
    state.switch_active_account(Some("account-b".to_string()));
    assert!(!state.expanded_snapshot_trade_ids.contains("trade-a"));

    state
        .expanded_snapshot_trade_ids
        .insert("trade-b".to_string());
    state.switch_active_account(Some("account-a".to_string()));
    assert!(state.expanded_snapshot_trade_ids.contains("trade-a"));
    assert!(!state.expanded_snapshot_trade_ids.contains("trade-b"));

    state.switch_active_account(Some("account-b".to_string()));
    assert!(state.expanded_snapshot_trade_ids.contains("trade-b"));
}

#[test]
fn journal_sync_status_is_scoped_by_account() {
    let mut state = JournalState::new_for_account(
        Some("account-a".to_string()),
        HashMap::new(),
        HashMap::new(),
    );
    state.sync_status = JournalSyncStatus {
        watermark_ms: Some(10_000),
        next_start_ms: Some(5_000),
        pages_loaded: 2,
        fills_loaded: 4_000,
        pagination_warning: Some("page boundary warning".to_string()),
        complete: false,
    };

    state.switch_active_account(Some("account-b".to_string()));
    assert_eq!(state.sync_status, JournalSyncStatus::default());

    state.sync_status = JournalSyncStatus {
        watermark_ms: Some(20_000),
        next_start_ms: None,
        pages_loaded: 1,
        fills_loaded: 42,
        pagination_warning: None,
        complete: true,
    };

    state.switch_active_account(Some("account-a".to_string()));
    assert_eq!(state.sync_status.pages_loaded, 2);
    assert_eq!(state.sync_status.fills_loaded, 4_000);
    assert_eq!(state.sync_status.next_start_ms, Some(5_000));
    assert_eq!(
        state.sync_status.pagination_warning.as_deref(),
        Some("page boundary warning")
    );

    state.switch_active_account(Some("account-b".to_string()));
    assert_eq!(state.sync_status.pages_loaded, 1);
    assert_eq!(state.sync_status.fills_loaded, 42);
    assert!(state.sync_status.complete);
}

#[test]
fn journal_clear_data_resets_sync_status() {
    let mut state = JournalState::new_for_account(
        Some("account-a".to_string()),
        HashMap::new(),
        HashMap::new(),
    );
    state.sync_status = JournalSyncStatus {
        watermark_ms: Some(10_000),
        next_start_ms: Some(5_000),
        pages_loaded: 2,
        fills_loaded: 4_000,
        pagination_warning: Some("page boundary warning".to_string()),
        complete: false,
    };

    state.clear_active_account_data();

    assert_eq!(state.sync_status, JournalSyncStatus::default());
}

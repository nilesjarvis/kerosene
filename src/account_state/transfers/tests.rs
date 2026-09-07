use super::*;

fn entry(provider: TransferProvider, id: &str, time: u64) -> TransferEntry {
    TransferEntry {
        provider,
        id: id.into(),
        time,
        direction: crate::account::transfers::TransferDirection::Deposit,
        asset: "BTC".into(),
        amount: "0.1".into(),
        source_chain: "Bitcoin".into(),
        destination_chain: "Hyperliquid".into(),
        source_address: Some("synthetic-source-wallet".into()),
        destination_address: None,
        protocol_address: None,
        source_tx: None,
        destination_tx: None,
        ledger_tx: None,
        status: "Discovered".into(),
        failed: false,
        fee: None,
        sweep_fee: None,
        source_confirmations: None,
        destination_confirmations: None,
    }
}

#[test]
fn switching_accounts_and_reconnecting_reject_old_results() {
    let mut state = TransferHistoryState::default();
    let old = state.begin("account-a").expect("start");
    assert!(state.begin("account-a").is_none());
    state.clear();
    let new = state.begin("account-a").expect("reconnect");
    assert_ne!(old, new);
    state.apply(
        "account-a",
        old,
        TransferProvider::Unit,
        Ok(TransferSnapshot::default()),
    );
    assert!(state.unit.loading);
    state.begin("account-b");
    state.apply(
        "account-a",
        new,
        TransferProvider::Unit,
        Ok(TransferSnapshot::default()),
    );
    assert!(!state.unit.loaded);
}

#[test]
fn providers_complete_independently_and_errors_retain_previous_success() {
    let mut state = TransferHistoryState::default();
    let generation = state.begin("account").expect("start");
    state.apply(
        "account",
        generation,
        TransferProvider::Hyperliquid,
        Ok(TransferSnapshot {
            next_start: Some(200),
            ..Default::default()
        }),
    );
    state.apply(
        "account",
        generation,
        TransferProvider::Unit,
        Err("Unit unavailable".into()),
    );
    assert!(!state.loading());
    assert!(state.native.loaded);
    assert!(!state.unit.loaded);
    let generation = state.begin("account").expect("retry");
    state.apply(
        "account",
        generation,
        TransferProvider::Hyperliquid,
        Err("Offline".into()),
    );
    state.apply(
        "account",
        generation,
        TransferProvider::Unit,
        Ok(TransferSnapshot::default()),
    );
    assert!(state.native.loaded);
    assert_eq!(state.native.next_start, Some(200));
    assert!(state.unit.error.is_none());
    assert!(state.unit.loaded);
}

#[test]
fn native_boundaries_merge_and_unit_snapshots_replace_pending_operations() {
    let mut state = TransferHistoryState::default();
    let first = state.begin("account").expect("start");
    state.apply(
        "account",
        first,
        TransferProvider::Hyperliquid,
        Ok(TransferSnapshot {
            entries: vec![entry(TransferProvider::Hyperliquid, "hl:1", 100)],
            ..Default::default()
        }),
    );
    state.apply(
        "account",
        first,
        TransferProvider::Unit,
        Ok(TransferSnapshot {
            entries: vec![entry(TransferProvider::Unit, "unit:unconfirmed", 200)],
            ..Default::default()
        }),
    );
    let second = state.begin("account").expect("refresh");
    state.apply(
        "account",
        second,
        TransferProvider::Hyperliquid,
        Ok(TransferSnapshot {
            entries: vec![
                entry(TransferProvider::Hyperliquid, "hl:1", 100),
                entry(TransferProvider::Hyperliquid, "hl:2", 300),
            ],
            ..Default::default()
        }),
    );
    let mut completed = entry(TransferProvider::Unit, "unit:confirmed", 200);
    completed.status = "Completed".into();
    state.apply(
        "account",
        second,
        TransferProvider::Unit,
        Ok(TransferSnapshot {
            entries: vec![completed],
            ..Default::default()
        }),
    );
    assert_eq!(
        state
            .entries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        ["hl:2", "unit:confirmed", "hl:1"]
    );
    assert_eq!(state.entries[1].status, "Completed");
    let third = state.begin("account").expect("refresh");
    state.apply(
        "account",
        third,
        TransferProvider::Unit,
        Err("Offline".into()),
    );
    assert_eq!(state.entries.len(), 3);
    state.clear();
    assert!(state.entries.is_empty());
    assert!(state.address.is_none());
    assert!(!state.loading());
}

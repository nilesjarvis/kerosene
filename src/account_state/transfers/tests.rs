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
        fee_asset: None,
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

#[test]
fn history_tabs_filter_before_pagination_and_keep_independent_details() {
    let bridges = TransferHistoryKind::DepositsWithdrawals;
    let transfers = TransferHistoryKind::Transfers;
    let mut state = TransferHistoryState::default();
    let generation = state.begin("account").expect("start");
    let entries = (0..122)
        .map(|i| {
            let mut row = entry(TransferProvider::Hyperliquid, &format!("hl:{i}"), i);
            if i % 2 == 0 {
                row.direction = crate::account::transfers::TransferDirection::Received;
            }
            row
        })
        .collect();
    state.apply(
        "account",
        generation,
        TransferProvider::Hyperliquid,
        Ok(TransferSnapshot {
            entries,
            ..Default::default()
        }),
    );
    assert_eq!(state.entries(bridges).count(), 61);
    assert_eq!(state.entries(transfers).count(), 61);
    let (index, row) = state.entries(transfers).next().expect("transfer");
    let id = row.id.clone();
    state.toggle_details(transfers, index);
    assert_eq!(state.view(transfers).expanded.as_deref(), Some(id.as_str()));
    state.toggle_details(bridges, index);
    assert!(state.view(bridges).expanded.is_none());
    state.change_page(bridges, true);
    state.change_page(bridges, true);
    assert_eq!(state.view(bridges).page, 1);
    assert_eq!(state.view(transfers).page, 0);
    assert_eq!(state.view(transfers).expanded.as_deref(), Some(id.as_str()));
    state.toggle_details(transfers, index);
    assert!(state.view(transfers).expanded.is_none());
    state.change_page(transfers, true);
    assert_eq!(
        state.entries(transfers).skip(TRANSFER_PAGE_SIZE).count(),
        11
    );
    assert_eq!(state.view(transfers).page, 1);
    state.change_page(transfers, false);
    state.change_page(transfers, false);
    assert_eq!(state.view(transfers).page, 0);
    state.clear();
    for kind in [bridges, transfers] {
        assert_eq!(state.view(kind).page, 0);
        assert!(state.view(kind).expanded.is_none());
        assert_eq!(state.entries(kind).count(), 0);
    }
}

#[test]
fn provider_refresh_clamps_each_tab_to_its_filtered_length() {
    let mut state = TransferHistoryState::default();
    let generation = state.begin("account").expect("start");
    let mut transfer = entry(TransferProvider::Hyperliquid, "hl:transfer", 200);
    transfer.direction = crate::account::transfers::TransferDirection::Sent;
    state.apply(
        "account",
        generation,
        TransferProvider::Hyperliquid,
        Ok(TransferSnapshot {
            entries: vec![transfer],
            ..Default::default()
        }),
    );
    state.view_mut(TransferHistoryKind::Transfers).page = 2;
    state
        .view_mut(TransferHistoryKind::DepositsWithdrawals)
        .page = 2;
    state.apply(
        "account",
        generation,
        TransferProvider::Unit,
        Ok(TransferSnapshot {
            entries: (0..51)
                .map(|i| entry(TransferProvider::Unit, &format!("unit:{i}"), i))
                .collect(),
            ..Default::default()
        }),
    );
    assert_eq!(state.view(TransferHistoryKind::Transfers).page, 0);
    assert_eq!(state.view(TransferHistoryKind::DepositsWithdrawals).page, 1);
}

use super::*;

#[test]
fn journal_snapshot_exposes_rankable_trades_and_redacted_reflections() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".into());
    terminal.journal.active_account_key = Some("private-account-key".to_string());
    terminal.journal.last_refresh_time = Some(1_234_567);
    terminal.journal.sync_status.complete = true;
    terminal.openrouter_api_key = "sk-or-private-journal-key".into();
    terminal.journal.trades.push(journal_trade(
        "internal-trade-id",
        "BTC",
        250.0,
        10.0,
        1_000,
    ));
    terminal.journal.entries.insert(
        "internal-trade-id".to_string(),
        crate::journal::JournalNote {
            open: "breakout thesis sk-or-private-journal-key".to_string(),
            close: "wallet 0xabc0000000000000000000000000000000000000".to_string(),
            cause_of_error: String::new(),
            tags: vec!["momentum".to_string()],
        },
    );

    let bytes = terminal.build_agent_snapshot().expect("snapshot");
    let text = String::from_utf8(bytes).expect("utf8");
    let value: Value = serde_json::from_str(&text).expect("json");
    let row = &value["_tool_data"]["journal"]["trades"][0];

    assert_eq!(value["journal"]["data_state"], "ready");
    assert_eq!(value["journal"]["total_trade_count"], 1);
    assert_eq!(row["symbol"], "BTC");
    assert_eq!(row["side"], "long");
    assert_eq!(row["gross_realized_pnl_usd"], 250.0);
    assert_eq!(row["fees_usd"], 10.0);
    assert_eq!(row["net_realized_pnl_usd"], 240.0);
    assert_eq!(row["return_on_entry_pct"], 24.0);
    assert_eq!(row["reflection"]["tags"][0], "momentum");
    assert!(
        row["reflection"]["open_thesis"]
            .as_str()
            .is_some_and(|note| note.contains("<redacted>"))
    );
    assert!(!text.contains("internal-trade-id"));
    assert!(!text.contains("private-account-key"));
    assert!(!text.contains("sk-or-private-journal-key"));
    assert!(!text.contains("0xabc0000000000000000000000000000000000000"));
}

#[test]
fn public_and_private_journal_status_preserve_availability_and_error_precedence() {
    let (mut terminal, _) = TradingTerminal::boot();
    let cases = [
        (
            false,
            true,
            false,
            false,
            false,
            true,
            "account_not_connected",
        ),
        (
            true,
            false,
            false,
            false,
            true,
            true,
            "account_not_connected",
        ),
        (true, true, true, true, false, true, "loading"),
        (true, true, false, true, false, true, "unavailable"),
        (true, true, false, false, false, true, "complete_empty"),
        (
            true,
            true,
            false,
            false,
            false,
            false,
            "not_loaded_or_partial",
        ),
        (true, true, true, true, true, true, "partial"),
        (true, true, false, false, true, false, "partial"),
        (true, true, false, true, true, true, "ready"),
        (true, true, false, false, true, true, "ready"),
    ];
    for (connected, active, loading, error, has_trades, complete, expected) in cases {
        terminal.connected_address = connected.then(|| "synthetic-account".into());
        terminal.journal.active_account_key = active.then(|| "synthetic-account".to_string());
        terminal.journal.loading = loading;
        terminal.journal.error = error.then(|| "synthetic-error".to_string());
        terminal.journal.sync_status.complete = complete;
        terminal.journal.trades = if has_trades {
            vec![journal_trade("synthetic-trade", "BTC", 1.0, 0.0, 1)]
        } else {
            Vec::new()
        };

        let value: Value =
            serde_json::from_slice(&terminal.build_agent_snapshot().expect("snapshot"))
                .expect("snapshot json");
        assert_eq!(value["journal"]["data_state"], expected);
        assert_eq!(value["_tool_data"]["journal"]["data_state"], expected);
    }
}

#[test]
fn capped_journal_selection_preserves_net_pnl_extremes() {
    let trades = vec![
        journal_trade("best", "BTC", 1_000.0, 0.0, 1),
        journal_trade("worst", "ETH", -900.0, 0.0, 2),
        journal_trade("middle-1", "SOL", 5.0, 0.0, 3),
        journal_trade("middle-2", "HYPE", 4.0, 0.0, 4),
        journal_trade("middle-3", "DOGE", 3.0, 0.0, 5),
        journal_trade("recent", "XRP", 2.0, 0.0, 6),
    ];

    let selected = journal_trade_selection_indexes(&trades, &HashMap::new(), 4);

    assert!(selected.contains(&0), "best trade should be retained");
    assert!(selected.contains(&1), "worst trade should be retained");
    assert_eq!(selected.len(), 4);
}

fn journal_trade(
    id: &str,
    coin: &str,
    pnl: f64,
    fee: f64,
    start_time: u64,
) -> crate::journal::AggregatedTrade {
    crate::journal::AggregatedTrade {
        id: id.to_string(),
        legacy_note_ids: Vec::new(),
        coin: coin.to_string(),
        start_time,
        end_time: Some(start_time + 60_000),
        max_position: 1.0,
        volume: 2_000.0,
        fee,
        pnl,
        status: "CLOSED".to_string(),
        fill_count: 2,
        avg_entry_price: 100.0,
        total_entry_notional: 1_000.0,
        total_entry_size: 10.0,
        is_long: true,
        basis_complete: true,
    }
}

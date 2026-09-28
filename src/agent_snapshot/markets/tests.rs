use super::*;

#[test]
fn public_market_cap_prioritizes_active_symbol_and_private_index_is_complete() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.all_mids.clear();
    terminal.all_mids_updated_at_ms.clear();
    for index in 0..300 {
        let symbol = format!("@{index}");
        terminal.all_mids.insert(symbol.clone(), index as f64 + 1.0);
        terminal.all_mids_updated_at_ms.insert(symbol, 123);
    }
    terminal.all_mids.insert("BTC".to_string(), 65_000.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), 456);

    let bytes = terminal.build_agent_snapshot().expect("snapshot");
    let value: Value = serde_json::from_slice(&bytes).expect("json");
    let public_markets = value["markets"]["markets"].as_array().expect("markets");
    let private_markets = value["_tool_data"]["markets"]["rows"]
        .as_array()
        .expect("private markets");

    assert_eq!(public_markets.len(), MAX_MARKETS);
    assert_eq!(public_markets[0]["symbol"], "BTC");
    assert_eq!(public_markets[0]["raw_symbol_is_sanitized"], false);
    assert_eq!(value["markets"]["coverage"]["returned_count"], MAX_MARKETS);
    assert_eq!(value["markets"]["coverage"]["total_count"], 301);
    assert_eq!(value["markets"]["coverage"]["truncated"], true);
    assert_eq!(private_markets.len(), 301);
}

#[test]
fn market_selection_preserves_private_order_and_identical_row_fields() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.account_data = None;
    terminal.exchange_symbols.clear();
    terminal.all_mids.clear();
    terminal.all_mids_updated_at_ms.clear();
    terminal.active_symbol = "Z-ACTIVE".to_string();
    terminal.active_symbol_display = "Z-ACTIVE".to_string();
    terminal.favourite_symbols = vec!["K299".to_string(), "K010".to_string()];
    for index in 0..300 {
        let symbol = format!("K{index:03}");
        terminal.all_mids.insert(symbol.clone(), index as f64 + 1.0);
        terminal.all_mids_updated_at_ms.insert(symbol, index + 1);
    }
    terminal.all_mids.insert("Z-ACTIVE".to_string(), 42.0);
    let original_order = terminal.all_mids.keys().cloned().collect::<Vec<_>>();

    let value: Value = serde_json::from_slice(&terminal.build_agent_snapshot().expect("snapshot"))
        .expect("snapshot json");
    let public = value["markets"]["markets"].as_array().expect("public rows");
    let private = value["_tool_data"]["markets"]["rows"]
        .as_array()
        .expect("private rows");
    assert_eq!(
        private
            .iter()
            .map(|row| row["symbol"].as_str().expect("symbol"))
            .collect::<Vec<_>>(),
        original_order
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    );
    let mut expected = vec![
        "Z-ACTIVE".to_string(),
        "K299".to_string(),
        "K010".to_string(),
    ];
    expected.extend(
        (0..300)
            .filter(|index| ![10, 299].contains(index))
            .map(|index| format!("K{index:03}")),
    );
    expected.truncate(MAX_MARKETS);
    assert_eq!(
        public
            .iter()
            .map(|row| row["symbol"].as_str().expect("symbol"))
            .collect::<Vec<_>>(),
        expected.iter().map(String::as_str).collect::<Vec<_>>()
    );
    for row in public {
        assert_eq!(
            Some(row),
            private
                .iter()
                .find(|candidate| candidate["symbol"] == row["symbol"])
        );
    }
    assert_eq!(
        public[2],
        json!({
            "symbol": "K010", "canonical_symbol": "K010", "display_symbol": "K010",
            "market_type": null, "category": null, "mid": 11.0, "updated_at_ms": 11,
            "favourite": true, "max_leverage": null, "only_isolated": null,
            "raw_symbol_is_sanitized": false,
        })
    );
}

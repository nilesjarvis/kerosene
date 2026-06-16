use super::*;

fn user_fill(coin: &str, side: &str, oid: u64) -> serde_json::Value {
    serde_json::json!({
        "coin": coin,
        "px": "100",
        "sz": "0.1",
        "side": side,
        "time": 1_u64,
        "oid": oid,
        "dir": "Open Long",
        "closedPnl": "0",
        "fee": "0.01"
    })
}

fn clearinghouse_with_position(coin: &str) -> serde_json::Value {
    serde_json::json!({
        "marginSummary": {
            "accountValue": "100",
            "totalNtlPos": "10",
            "totalMarginUsed": "1"
        },
        "crossMarginSummary": null,
        "crossMaintenanceMarginUsed": null,
        "withdrawable": "99",
        "assetPositions": [{
            "position": {
                "coin": coin,
                "szi": "1",
                "entryPx": "10",
                "positionValue": "10",
                "unrealizedPnl": "0",
                "liquidationPx": null,
                "leverage": {
                    "type": "cross",
                    "value": 1
                },
                "marginUsed": "1",
                "cumFunding": null
            },
            "liquidationPx": null
        }]
    })
}

#[test]
fn user_fills_parser_preserves_canonical_market_symbols_and_wire_sides() {
    let target = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let payload = serde_json::json!({
        "user": target,
        "isSnapshot": true,
        "fills": [
            user_fill("BTC", "B", 1),
            user_fill("flx:BTC", "B", 2),
            user_fill("@107", "A", 3),
            user_fill("#950", "A", 4)
        ]
    });

    let Some((source_addr, WsUserData::Fills { fills, is_snapshot })) =
        parse_user_stream_message("userFills", &payload, Some(target), None)
    else {
        panic!("expected user fills update");
    };

    assert_eq!(source_addr.as_deref(), Some(target));
    assert!(is_snapshot);
    let parsed: Vec<(&str, &str)> = fills
        .iter()
        .map(|fill| (fill.coin.as_str(), fill.side.as_str()))
        .collect();
    assert_eq!(
        parsed,
        vec![("BTC", "B"), ("flx:BTC", "B"), ("@107", "A"), ("#950", "A")]
    );
}

#[test]
fn all_mids_parser_drops_invalid_prices() {
    let payload = serde_json::json!({
        "mids": {
            "BTC": "100.5",
            "BAD": "not-a-price",
            "NAN": "NaN",
            "INF": "inf",
            "ZERO": "0",
            "NEG": "-1"
        }
    });

    let Some((source_addr, WsUserData::AllMids(mids))) =
        parse_user_stream_message("allMids", &payload, None, Some("0xabc".to_string()))
    else {
        panic!("expected all mids update");
    };

    assert_eq!(source_addr.as_deref(), Some("0xabc"));
    assert_eq!(mids.get("BTC"), Some(&100.5));
    assert!(!mids.contains_key("BAD"));
    assert!(!mids.contains_key("NAN"));
    assert!(!mids.contains_key("INF"));
    assert!(!mids.contains_key("ZERO"));
    assert!(!mids.contains_key("NEG"));
}

#[test]
fn all_dex_positions_prefixes_hip3_position_coins() {
    let target = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let payload = serde_json::json!({
        "user": target,
        "clearinghouseStates": [
            ["", clearinghouse_with_position("BTC")],
            ["xyz", clearinghouse_with_position("NVDA")]
        ]
    });

    let Some((
        source_addr,
        WsUserData::AllDexPositions {
            states_by_dex,
            all_positions,
            position_details,
            ..
        },
    )) = parse_user_stream_message("allDexsClearinghouseState", &payload, Some(target), None)
    else {
        panic!("expected all-dex positions update");
    };

    assert_eq!(source_addr.as_deref(), Some(target));
    assert_eq!(all_positions[0].position.coin, "BTC");
    assert_eq!(all_positions[1].position.coin, "xyz:NVDA");
    assert_eq!(
        states_by_dex["xyz"].asset_positions[0].position.coin,
        "xyz:NVDA"
    );
    assert_eq!(position_details[1].dex, "xyz");
    assert_eq!(position_details[1].asset_position.position.coin, "xyz:NVDA");
}

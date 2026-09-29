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

#[test]
fn private_array_events_preserve_account_matching_and_reject_partial_payloads() {
    let target = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let other = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    for (channel, path, mut payload) in [
        (
            "openOrders",
            "/orders",
            serde_json::json!({ "orders": [{
                "coin": "BTC", "side": "B", "limitPx": "100", "sz": "1",
                "oid": 7, "timestamp": 10, "unknown": [null, true]
            }] }),
        ),
        (
            "userFills",
            "/fills",
            serde_json::json!({ "fills": [user_fill("BTC", "B", 7)] }),
        ),
        (
            "spotState",
            "/spotState/balances",
            serde_json::json!({ "spotState": { "balances": [{
                "coin": "USDC", "total": "100", "hold": "1", "entryNtl": "99"
            }] } }),
        ),
    ] {
        payload["user"] = serde_json::json!(format!(" {} ", target.to_uppercase()));
        let (source, update) = parse_user_stream_message(channel, &payload, Some(target), None)
            .expect("valid targeted payload");
        assert_eq!(source.as_deref(), Some(target));
        match update {
            WsUserData::OpenOrders { dex, orders } => {
                assert!(dex.is_empty());
                assert_eq!(orders[0].oid, 7);
                assert!(orders[0].reduce_only.is_none());
            }
            WsUserData::Fills { fills, is_snapshot } => {
                assert!(!is_snapshot);
                assert_eq!(fills[0].oid, Some(7));
                assert!(fills[0].hash.is_none());
            }
            WsUserData::SpotBalances(balances) => {
                assert_eq!(balances[0].total, "100");
                assert!(balances[0].token.is_none());
            }
            _ => panic!("unexpected private array event"),
        }
        assert!(parse_user_stream_message(channel, &payload, Some(other), None).is_none());
        assert!(parse_user_stream_message(channel, &payload, None, None).is_none());

        payload
            .pointer_mut(path)
            .expect("array field")
            .as_array_mut()
            .expect("array payload")
            .push(Value::Null);
        assert!(parse_user_stream_message(channel, &payload, Some(target), None).is_none());

        *payload.pointer_mut(path).expect("array field") = serde_json::json!([]);
        assert!(parse_user_stream_message(channel, &payload, Some(target), None).is_some());
        payload
            .as_object_mut()
            .expect("payload object")
            .remove("user");
        assert!(parse_user_stream_message(channel, &payload, Some(target), None).is_none());
    }
}

#[test]
fn all_dex_positions_keep_partial_entries_duplicates_and_last_main_state() {
    let target = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let payload = serde_json::json!({
        "user": target,
        "clearinghouseStates": [
            null, ["broken", {}], ["incomplete"],
            ["", clearinghouse_with_position("BTC")],
            ["xyz", clearinghouse_with_position("NVDA")],
            ["xyz", clearinghouse_with_position("TSLA"), "ignored"],
            [false, clearinghouse_with_position("ETH")],
            ["", {}]
        ]
    });
    let Some((
        _,
        WsUserData::AllDexPositions {
            main_state,
            states_by_dex,
            all_positions,
            position_details,
        },
    )) = parse_user_stream_message("allDexsClearinghouseState", &payload, Some(target), None)
    else {
        panic!("valid entries must survive malformed neighboring entries");
    };
    assert_eq!(main_state.asset_positions[0].position.coin, "ETH");
    assert_eq!(states_by_dex.len(), 2);
    assert_eq!(states_by_dex[""].asset_positions[0].position.coin, "ETH");
    assert_eq!(
        states_by_dex["xyz"].asset_positions[0].position.coin,
        "xyz:TSLA"
    );
    let symbols: Vec<_> = all_positions
        .iter()
        .map(|entry| entry.position.coin.as_str())
        .collect();
    assert_eq!(symbols, ["BTC", "xyz:NVDA", "xyz:TSLA", "ETH"]);
    let details: Vec<_> = position_details
        .iter()
        .map(|entry| {
            (
                entry.dex.as_str(),
                entry.asset_position.position.coin.as_str(),
            )
        })
        .collect();
    assert_eq!(
        details,
        [
            ("", "BTC"),
            ("xyz", "xyz:NVDA"),
            ("xyz", "xyz:TSLA"),
            ("", "ETH")
        ]
    );

    let no_main = serde_json::json!({
        "user": target,
        "clearinghouseStates": [["xyz", clearinghouse_with_position("NVDA")], ["", {}]]
    });
    assert!(
        parse_user_stream_message("allDexsClearinghouseState", &no_main, Some(target), None)
            .is_none()
    );
}

#[test]
fn all_mids_rejects_non_string_prices_before_numeric_filtering() {
    for malformed in [
        serde_json::json!(100),
        Value::Null,
        serde_json::json!(false),
    ] {
        let payload = serde_json::json!({ "mids": { "BTC": "100", "BAD": malformed } });
        assert!(parse_user_stream_message("allMids", &payload, None, None).is_none());
    }
}

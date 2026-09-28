use super::*;

#[test]
fn portfolio_state_all_dexes_parses_and_normalizes_dex_positions() {
    let raw = serde_json::json!({
        "clearinghouseState": {
            "native": clearinghouse_state_json("BTC"),
            "xyz": clearinghouse_state_json("MSFT")
        },
        "spotClearinghouseState": {
            "balances": [
                {
                    "coin": "USDC",
                    "token": 0,
                    "total": "10",
                    "hold": "0",
                    "entryNtl": "0"
                }
            ]
        },
        "userAbstraction": "unifiedAccount"
    });

    let portfolio = parse_portfolio_state(raw).expect("portfolio parses");
    assert_eq!(
        portfolio.account_abstraction(),
        AccountAbstractionMode::UnifiedAccount
    );
    let (native, by_dex, hip3_states) = portfolio
        .clearinghouses_for_scope(&AccountDataFetchScope::hip3_dex("xyz"))
        .expect("clearinghouses parse");

    assert_eq!(native.asset_positions[0].position.coin, "BTC");
    assert_eq!(by_dex["xyz"].asset_positions[0].position.coin, "xyz:MSFT");
    assert_eq!(hip3_states[0].asset_positions[0].position.coin, "xyz:MSFT");
    assert_eq!(
        portfolio
            .spot_clearinghouse()
            .expect("spot parses")
            .balances[0]
            .coin,
        "USDC"
    );
}

#[test]
fn portfolio_scope_retains_dex_result_order_and_repeated_entries() {
    let portfolio = parse_portfolio_state(serde_json::json!({
        "clearinghouseState": {
            "native": clearinghouse_state_json("BTC"),
            "xyz": clearinghouse_state_json("MSFT"),
            "flx": clearinghouse_state_json("GOLD")
        },
        "spotClearinghouseState": {"balances": []}
    }))
    .expect("portfolio parses");
    let scope = AccountDataFetchScope::AllMarkets {
        hip3_dexes: ["xyz", "missing", "flx", "xyz"]
            .into_iter()
            .map(str::to_string)
            .collect(),
    };
    let (native, by_dex, hip3) = portfolio
        .clearinghouses_for_scope(&scope)
        .expect("selected DEX states parse");
    assert_eq!(native.asset_positions[0].position.coin, "BTC");
    assert_eq!(by_dex.len(), 3);
    assert_eq!(by_dex["xyz"].asset_positions[0].position.coin, "xyz:MSFT");
    assert_eq!(by_dex["flx"].asset_positions[0].position.coin, "flx:GOLD");
    assert_eq!(
        hip3.iter()
            .map(|state| state.asset_positions[0].position.coin.as_str())
            .collect::<Vec<_>>(),
        ["xyz:MSFT", "flx:GOLD", "xyz:MSFT"]
    );
}

#[test]
fn portfolio_state_direct_clearinghouse_shape_parses_native_state() {
    let raw = serde_json::json!({
        "clearinghouseState": clearinghouse_state_json("ETH"),
        "spotClearinghouseState": { "balances": [] },
        "userAbstraction": "default"
    });

    let portfolio = parse_portfolio_state(raw).expect("portfolio parses");
    let (native, by_dex, hip3_states) = portfolio
        .clearinghouses_for_scope(&AccountDataFetchScope::default())
        .expect("clearinghouse parses");

    assert_eq!(native.asset_positions[0].position.coin, "ETH");
    assert_eq!(by_dex.len(), 1);
    assert!(hip3_states.is_empty());
}

#[test]
fn batch_portfolio_states_parses_successes_and_failures() {
    let raw = serde_json::json!({
        "successful_states": [
            [
                "0x0000000000000000000000000000000000000001",
                {
                    "clearinghouseState": clearinghouse_state_json("BTC"),
                    "spotClearinghouseState": { "balances": [] },
                    "userAbstraction": "default"
                }
            ]
        ],
        "failed_wallets": ["0x0000000000000000000000000000000000000002"]
    });

    let batch = parse_batch_portfolio_states(raw).expect("batch parses");

    assert_eq!(batch.successful_states.len(), 1);
    assert_eq!(
        batch.successful_states[0].0,
        "0x0000000000000000000000000000000000000001"
    );
    assert_eq!(
        batch.failed_wallets,
        vec!["0x0000000000000000000000000000000000000002"]
    );
}

#[test]
fn selected_dex_portfolios_merge_without_all_dexes() {
    let native_raw = serde_json::json!({
        "clearinghouseState": clearinghouse_state_json("BTC"),
        "spotClearinghouseState": { "balances": [] },
        "userAbstraction": "default"
    });
    let dex_raw = serde_json::json!({
        "clearinghouseState": clearinghouse_state_json("MSFT"),
        "spotClearinghouseState": { "balances": [] },
        "userAbstraction": "default"
    });

    let portfolio = merge_native_and_dex_portfolio_states(native_raw, dex_raw, "xyz")
        .expect("portfolio merges");
    let (native, by_dex, hip3_states) = portfolio
        .clearinghouses_for_scope(&AccountDataFetchScope::hip3_dex("xyz"))
        .expect("clearinghouses parse");

    assert_eq!(native.asset_positions[0].position.coin, "BTC");
    assert_eq!(by_dex.len(), 2);
    assert_eq!(hip3_states[0].asset_positions[0].position.coin, "xyz:MSFT");
}

#[test]
fn hydromancer_batch_chunk_size_matches_api_limits() {
    assert_eq!(
        hydromancer_portfolio_chunk_size(&AccountDataFetchScope::default()),
        100
    );
    assert_eq!(
        hydromancer_portfolio_chunk_size(&AccountDataFetchScope::hip3_dex("xyz")),
        500
    );
}

#[test]
fn portfolio_state_debug_redacts_raw_api_values() {
    let portfolio = HydromancerPortfolioState {
        clearinghouse_state: serde_json::json!({
            "wallet": "0xabc0000000000000000000000000000000000000",
            "client_secret": "portfolio-secret"
        }),
        spot_clearinghouse_state: serde_json::json!({
            "balances": ["spot-secret"]
        }),
        user_abstraction: Value::String("unifiedAccount".to_string()),
    };

    let rendered = format!("{portfolio:?}");

    assert!(rendered.contains("HydromancerPortfolioState"));
    assert_eq!(rendered.matches("<redacted>").count(), 3);
    for secret in [
        "0xabc0000000000000000000000000000000000000",
        "portfolio-secret",
        "spot-secret",
        "unifiedAccount",
    ] {
        assert!(!rendered.contains(secret), "debug leaked {secret}");
    }
}

#[test]
fn portfolio_fields_preserve_missing_errors_null_values_and_abstraction_defaults() {
    for (raw, expected) in [
        (Value::Null, "portfolioState missing clearinghouseState"),
        (
            serde_json::json!({}),
            "portfolioState missing clearinghouseState",
        ),
        (
            serde_json::json!({"spotClearinghouseState": null}),
            "portfolioState missing clearinghouseState",
        ),
        (
            serde_json::json!({"clearinghouseState": null}),
            "portfolioState missing spotClearinghouseState",
        ),
    ] {
        assert_eq!(parse_portfolio_state(raw).err().as_deref(), Some(expected));
    }

    let portfolio = parse_portfolio_state(serde_json::json!({
        "clearinghouseState": null,
        "spotClearinghouseState": null,
    }))
    .expect("present null fields are retained for downstream validation");
    assert_eq!(portfolio.clearinghouse_state, Value::Null);
    assert_eq!(portfolio.spot_clearinghouse_state, Value::Null);
    assert_eq!(
        portfolio.account_abstraction(),
        AccountAbstractionMode::Default
    );

    for abstraction in [
        Value::Null,
        serde_json::json!(false),
        serde_json::json!({"mode": 3}),
    ] {
        let portfolio = parse_portfolio_state(serde_json::json!({
            "clearinghouseState": {"nested": [1, {"value": "kept"}]},
            "spotClearinghouseState": [2, 3],
            "userAbstraction": abstraction,
        }))
        .expect("parser retains raw fields without eager conversion");
        assert_eq!(
            portfolio.clearinghouse_state,
            serde_json::json!({"nested": [1, {"value": "kept"}]})
        );
        assert_eq!(
            portfolio.spot_clearinghouse_state,
            serde_json::json!([2, 3])
        );
        assert_eq!(portfolio.user_abstraction, abstraction);
        assert_eq!(
            portfolio.account_abstraction(),
            AccountAbstractionMode::Unknown(abstraction.to_string())
        );
    }
}

#[test]
fn merged_portfolio_keeps_native_spot_and_abstraction_fields() {
    let native = serde_json::json!({
        "clearinghouseState": {"source": "native"},
        "spotClearinghouseState": {"source": "native spot"},
        "userAbstraction": null,
    });
    let dex = serde_json::json!({
        "clearinghouseState": {"source": "dex"},
        "spotClearinghouseState": {"source": "dex spot"},
        "userAbstraction": "unifiedAccount",
    });

    let portfolio = merge_native_and_dex_portfolio_states(native, dex, "xyz")
        .expect("raw portfolio fields merge");

    assert_eq!(
        portfolio.clearinghouse_state,
        serde_json::json!({
            "native": {"source": "native"},
            "xyz": {"source": "dex"},
        })
    );
    assert_eq!(
        portfolio.spot_clearinghouse_state,
        serde_json::json!({"source": "native spot"})
    );
    assert_eq!(portfolio.user_abstraction, Value::Null);
    assert_eq!(
        merge_native_and_dex_portfolio_states(
            serde_json::json!({"spotClearinghouseState": {}}),
            serde_json::json!({"clearinghouseState": {}}),
            "xyz",
        )
        .err()
        .as_deref(),
        Some("portfolioState missing clearinghouseState")
    );
}

#[test]
fn batch_fields_preserve_alias_precedence_order_duplicates_and_raw_states() {
    let batch = parse_batch_portfolio_states(serde_json::json!({
        "successful_states": [
            ["first", {"nested": [1, {"value": "kept"}]}],
            ["second", null],
            ["first", [2, 3]],
        ],
        "successfulStates": [["ignored", false]],
        "failed_wallets": ["second", 7, null, {}, [], true, "first", "second"],
        "failedWallets": ["ignored"],
    }))
    .expect("primary batch fields parse");

    assert_eq!(
        batch.successful_states,
        vec![
            (
                "first".to_string(),
                serde_json::json!({"nested": [1, {"value": "kept"}]})
            ),
            ("second".to_string(), Value::Null),
            ("first".to_string(), serde_json::json!([2, 3])),
        ]
    );
    assert_eq!(batch.failed_wallets, ["second", "first", "second"]);

    let batch = parse_batch_portfolio_states(serde_json::json!({
        "successfulStates": [["", false]],
        "failedWallets": ["first", null, "second"],
    }))
    .expect("camel-case aliases are used when primary fields are absent");
    assert_eq!(
        batch.successful_states,
        [(String::new(), Value::Bool(false))]
    );
    assert_eq!(batch.failed_wallets, ["first", "second"]);
}

#[test]
fn batch_aliases_do_not_replace_present_primary_fields_of_the_wrong_type() {
    for primary in [
        Value::Null,
        Value::Bool(false),
        serde_json::json!({}),
        serde_json::json!("text"),
    ] {
        let raw = serde_json::json!({
            "successful_states": primary,
            "successfulStates": [],
        });
        assert_eq!(
            parse_batch_portfolio_states(raw).err().as_deref(),
            Some("batchPortfolioStates missing successful_states")
        );

        let batch = parse_batch_portfolio_states(serde_json::json!({
            "successful_states": [],
            "successfulStates": [["ignored", null]],
            "failed_wallets": primary,
            "failedWallets": ["ignored"],
        }))
        .expect("malformed optional failures default to empty");
        assert!(batch.successful_states.is_empty());
        assert!(batch.failed_wallets.is_empty());
    }
}

#[test]
fn batch_tuple_errors_preserve_validation_order_and_reject_partial_results() {
    for (items, expected) in [
        (
            serde_json::json!([7]),
            "batchPortfolioStates successful state was not a tuple",
        ),
        (
            serde_json::json!([[]]),
            "batchPortfolioStates successful state tuple had wrong length",
        ),
        (
            serde_json::json!([[7]]),
            "batchPortfolioStates successful state tuple had wrong length",
        ),
        (
            serde_json::json!([[7, {}, null]]),
            "batchPortfolioStates successful state tuple had wrong length",
        ),
        (
            serde_json::json!([[7, {}]]),
            "batchPortfolioStates successful state missing address",
        ),
        (
            serde_json::json!([["valid", {}], false]),
            "batchPortfolioStates successful state was not a tuple",
        ),
    ] {
        let raw = serde_json::json!({"successful_states": items});
        assert_eq!(
            parse_batch_portfolio_states(raw).err().as_deref(),
            Some(expected)
        );
    }
    for raw in [
        Value::Null,
        serde_json::json!([]),
        serde_json::json!({"failed_wallets": []}),
    ] {
        assert_eq!(
            parse_batch_portfolio_states(raw).err().as_deref(),
            Some("batchPortfolioStates missing successful_states")
        );
    }
}

#[test]
fn clearinghouse_getters_preserve_native_precedence_and_scope_filtering() {
    for (raw, expected) in [
        (
            Value::Null,
            "portfolioState clearinghouseState was not an object",
        ),
        (
            serde_json::json!([]),
            "portfolioState clearinghouseState was not an object",
        ),
        (
            serde_json::json!({}),
            "portfolioState missing native clearinghouseState",
        ),
        (
            serde_json::json!({"native": null, "": clearinghouse_state_json("ETH")}),
            " clearinghouseState deserialize failed: invalid type: null, expected struct ClearinghouseState",
        ),
        (
            serde_json::json!({"marginSummary": null, "native": clearinghouse_state_json("BTC")}),
            " clearinghouseState deserialize failed: invalid type: null, expected struct MarginSummary",
        ),
    ] {
        let portfolio = parse_portfolio_state(serde_json::json!({
            "clearinghouseState": raw,
            "spotClearinghouseState": {"balances": []}
        }))
        .expect("portfolio fields are present");
        assert_eq!(
            portfolio
                .clearinghouses_for_scope(&AccountDataFetchScope::hip3_dex("xyz"))
                .err()
                .as_deref(),
            Some(expected)
        );
    }

    let portfolio = parse_portfolio_state(serde_json::json!({
        "clearinghouseState": {"": clearinghouse_state_json("ETH"), "xyz": null},
        "spotClearinghouseState": {"balances": []}
    }))
    .expect("portfolio parses");
    let (native, by_dex, hip3) = portfolio
        .clearinghouses_for_scope(&AccountDataFetchScope::hip3_dex("flx"))
        .expect("missing selected DEX is skipped, malformed unselected DEX is ignored");
    assert_eq!(native.asset_positions[0].position.coin, "ETH");
    assert_eq!(by_dex.len(), 1);
    assert!(hip3.is_empty());
    assert_eq!(
        portfolio
            .clearinghouses_for_scope(&AccountDataFetchScope::hip3_dex("xyz"))
            .err()
            .as_deref(),
        Some(
            "xyz clearinghouseState deserialize failed: invalid type: null, expected struct ClearinghouseState"
        )
    );
}

#[test]
fn clearinghouse_getter_preserves_nested_deserialization_errors() {
    for (pointer, value, expected) in [
        (
            "/marginSummary/accountValue",
            serde_json::json!(100),
            "invalid type: integer `100`, expected a string",
        ),
        (
            "/assetPositions",
            Value::Null,
            "invalid type: null, expected a sequence",
        ),
        (
            "/assetPositions/0/position/leverage/value",
            serde_json::json!(-1),
            "invalid value: integer `-1`, expected u32",
        ),
        (
            "/assetPositions/0/position/leverage/value",
            serde_json::json!(4294967296_u64),
            "invalid value: integer `4294967296`, expected u32",
        ),
        (
            "/assetPositions/0/position/leverage/value",
            serde_json::json!(3.5),
            "invalid type: floating point `3.5`, expected u32",
        ),
        (
            "/assetPositions/0/position/liquidationPx",
            serde_json::json!(false),
            "invalid type: boolean `false`, expected a string",
        ),
    ] {
        let mut clearinghouse = clearinghouse_state_json("BTC");
        *clearinghouse
            .pointer_mut(pointer)
            .expect("fixture field exists") = value;
        let portfolio = parse_portfolio_state(serde_json::json!({
            "clearinghouseState": clearinghouse,
            "spotClearinghouseState": {"balances": []}
        }))
        .expect("portfolio fields are present");
        assert_eq!(
            portfolio
                .clearinghouses_for_scope(&AccountDataFetchScope::default())
                .err(),
            Some(format!(
                " clearinghouseState deserialize failed: {expected}"
            ))
        );
    }
}

#[test]
fn spot_getter_preserves_deserialization_errors() {
    for (raw, expected) in [
        (
            Value::Null,
            "invalid type: null, expected struct SpotClearinghouseState",
        ),
        (serde_json::json!({}), "missing field `balances`"),
        (
            serde_json::json!({"balances": null}),
            "invalid type: null, expected a sequence",
        ),
        (
            serde_json::json!({"balances": [], "portfolioMarginEnabled": null}),
            "invalid type: null, expected a boolean",
        ),
        (
            serde_json::json!({"balances": [], "tokenToAvailableAfterMaintenance": [[0]]}),
            "invalid length 1, expected a tuple of size 2",
        ),
        (
            serde_json::json!({"balances": [], "tokenToAvailableAfterMaintenance": [[0, "5", "extra"]]}),
            "invalid length 3, expected fewer elements in array",
        ),
        (
            serde_json::json!({"balances": [], "tokenToAvailableAfterMaintenance": [[-1, "5"]]}),
            "invalid value: integer `-1`, expected u32",
        ),
    ] {
        let portfolio = parse_portfolio_state(serde_json::json!({
            "clearinghouseState": null,
            "spotClearinghouseState": raw
        }))
        .expect("portfolio fields are present");
        assert_eq!(
            portfolio.spot_clearinghouse().err(),
            Some(format!(
                "spotClearinghouseState deserialize failed: {expected}"
            ))
        );
    }
}

#[test]
fn portfolio_getters_preserve_defaults_and_independent_owned_results() {
    let mut native_raw = clearinghouse_state_json("BTC");
    let native_object = native_raw.as_object_mut().expect("fixture is an object");
    native_object.remove("crossMarginSummary");
    native_object.remove("crossMaintenanceMarginUsed");
    native_raw["assetPositions"][0]["position"]
        .as_object_mut()
        .expect("position is an object")
        .remove("marginUsed");
    let portfolio = parse_portfolio_state(serde_json::json!({
        "clearinghouseState": {"native": native_raw, "xyz": clearinghouse_state_json("MSFT")},
        "spotClearinghouseState": {"balances": [{"coin": "USDC", "total": "10", "hold": "2", "entryNtl": "1"}]}
    })).expect("portfolio parses");
    let scope = AccountDataFetchScope::hip3_dex("xyz");
    let (mut native, mut by_dex, mut hip3) = portfolio
        .clearinghouses_for_scope(&scope)
        .expect("states parse");
    assert!(native.cross_margin_summary.is_none());
    assert!(native.cross_maintenance_margin_used.is_none());
    let position = &native.asset_positions[0];
    assert!(position.liquidation_px.is_none());
    assert!(position.position.margin_used.is_empty());
    assert!(position.position.cum_funding.is_none());
    assert_eq!(native.margin_summary.account_value, "100");
    assert_eq!(native.margin_summary.total_margin_used, "5");
    assert_eq!(native.withdrawable, "95");
    native.asset_positions.clear();
    hip3[0].asset_positions.clear();
    assert_eq!(by_dex[""].asset_positions[0].position.coin, "BTC");
    assert_eq!(by_dex["xyz"].asset_positions[0].position.coin, "xyz:MSFT");
    by_dex.clear();
    let (native, by_dex, hip3) = portfolio
        .clearinghouses_for_scope(&scope)
        .expect("repeated access succeeds");
    assert_eq!(native.asset_positions[0].position.coin, "BTC");
    assert_eq!(by_dex.len(), 2);
    assert_eq!(hip3[0].asset_positions[0].position.coin, "xyz:MSFT");

    let mut spot = portfolio.spot_clearinghouse().expect("spot parses");
    assert!(!spot.portfolio_margin_enabled);
    assert!(spot.portfolio_margin_ratio.is_none());
    assert!(spot.token_to_available_after_maintenance.is_none());
    assert!(spot.balances[0].token.is_none());
    assert!(spot.balances[0].supplied.is_none());
    assert_eq!(spot.balances[0].total, "10");
    assert_eq!(spot.balances[0].hold, "2");
    assert_eq!(spot.balances[0].entry_ntl, "1");
    spot.balances.clear();
    assert_eq!(
        portfolio
            .spot_clearinghouse()
            .expect("repeated spot access succeeds")
            .balances[0]
            .coin,
        "USDC"
    );
}

fn clearinghouse_state_json(coin: &str) -> Value {
    serde_json::json!({
        "marginSummary": {
            "accountValue": "100",
            "totalNtlPos": "10",
            "totalMarginUsed": "5"
        },
        "crossMarginSummary": {
            "accountValue": "100",
            "totalNtlPos": "10",
            "totalMarginUsed": "5"
        },
        "crossMaintenanceMarginUsed": "1",
        "withdrawable": "95",
        "assetPositions": [
            {
                "position": {
                    "coin": coin,
                    "szi": "1",
                    "entryPx": "10",
                    "positionValue": "10",
                    "unrealizedPnl": "0",
                    "liquidationPx": null,
                    "leverage": {
                        "type": "cross",
                        "value": 3
                    },
                    "marginUsed": "3"
                }
            }
        ]
    })
}

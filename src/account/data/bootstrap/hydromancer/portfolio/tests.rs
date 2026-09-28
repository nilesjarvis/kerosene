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

use super::*;
use crate::app_state::sensitive_string;
use crate::timeframe::Timeframe;

fn minute_candle(
    open_time: u64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
) -> Candle {
    Candle {
        open_time,
        close_time: open_time + Timeframe::M1.duration_ms() - 1,
        open,
        high,
        low,
        close,
        volume,
    }
}

fn summary(hash: &str) -> SchwabAccountSummary {
    SchwabAccountSummary {
        account_number: None,
        account_hash: hash.to_string(),
        account_type: None,
        cash_balance: None,
        buying_power: None,
        liquidation_value: None,
        positions: Vec::new(),
    }
}

fn linked(hash: &str) -> SchwabLinkedAccount {
    SchwabLinkedAccount {
        account_number: None,
        hash_value: hash.to_string(),
    }
}

#[test]
fn schwab_symbol_key_normalizes_and_round_trips() {
    assert_eq!(
        schwab_symbol_key(" brk/b "),
        Some("schwab:BRK/B".to_string())
    );
    assert_eq!(schwab_symbol_key("aapl"), Some("schwab:AAPL".to_string()));
    assert_eq!(schwab_symbol_key("$&!"), None);
    assert_eq!(schwab_symbol_from_key("schwab:AAPL"), Some("AAPL"));
    assert_eq!(schwab_symbol_from_key("schwab:"), None);
    assert_eq!(schwab_symbol_from_key("AAPL"), None);
    assert!(is_schwab_symbol_key("schwab:MSFT"));
    assert!(!is_schwab_symbol_key("BTC"));
    assert_eq!(
        schwab_display_symbol("schwab:MSFT"),
        Some("MSFT".to_string())
    );
    assert_eq!(schwab_display_symbol("BTC"), None);
}

#[test]
fn mask_identifier_keeps_last_four_characters() {
    assert_eq!(mask_identifier("123456789"), "...6789");
    assert_eq!(mask_identifier("42"), "...42");
    assert_eq!(mask_identifier("   "), "No account");
}

#[test]
fn aggregate_candles_merges_base_bars_into_target_buckets() {
    let minute = Timeframe::M1.duration_ms();
    let target = Timeframe::M3.duration_ms();
    let start = target * 1_000;
    let candles = vec![
        minute_candle(start, 10.0, 12.0, 9.0, 11.0, 1.0),
        minute_candle(start + minute, 11.0, 15.0, 10.0, 14.0, 2.0),
        minute_candle(start + 2 * minute, 14.0, 14.5, 8.0, 9.0, 3.0),
        // Gap: the next bar skips one full target bucket.
        minute_candle(start + 2 * target, 20.0, 21.0, 19.0, 20.5, 4.0),
    ];

    let aggregated = aggregate_candles(candles, target);

    assert_eq!(aggregated.len(), 2);
    let first = &aggregated[0];
    assert_eq!(first.open_time, start);
    assert_eq!(first.close_time, start + target - 1);
    assert_eq!(first.open, 10.0);
    assert_eq!(first.high, 15.0);
    assert_eq!(first.low, 8.0);
    assert_eq!(first.close, 9.0);
    assert_eq!(first.volume, 6.0);
    let second = &aggregated[1];
    assert_eq!(second.open_time, start + 2 * target);
    assert_eq!(second.volume, 4.0);
}

#[test]
fn aggregate_candles_passes_through_degenerate_inputs() {
    assert!(aggregate_candles(Vec::new(), Timeframe::M3.duration_ms()).is_empty());

    let candles = vec![minute_candle(60_000, 1.0, 2.0, 0.5, 1.5, 1.0)];
    let unchanged = aggregate_candles(candles.clone(), 0);
    assert_eq!(unchanged.len(), 1);
    assert_eq!(unchanged[0].open_time, candles[0].open_time);
}

#[test]
fn schwab_price_history_params_cover_supported_timeframes() {
    let native = schwab_price_history_params(Timeframe::M5).expect("5m supported");
    assert_eq!(native.base_interval_ms, Timeframe::M5.duration_ms());
    assert_eq!(native.frequency_type, "minute");

    let aggregated = schwab_price_history_params(Timeframe::M3).expect("3m supported");
    assert_eq!(aggregated.base_interval_ms, Timeframe::M1.duration_ms());

    let hourly = schwab_price_history_params(Timeframe::H4).expect("4h supported");
    assert_eq!(hourly.base_interval_ms, Timeframe::M30.duration_ms());

    let daily = schwab_price_history_params(Timeframe::D1).expect("1d supported");
    assert_eq!(daily.frequency_type, "daily");
    assert_eq!(daily.base_interval_ms, Timeframe::D1.duration_ms());

    assert!(schwab_price_history_params(Timeframe::Tick).is_err());
    assert!(schwab_price_history_params(Timeframe::S1).is_err());
}

#[test]
fn schwab_price_candle_conversion_validates_values() {
    let interval = Timeframe::M5.duration_ms();
    let negative_volume = SchwabPriceCandle {
        datetime: 1_000,
        open: 1.0,
        high: 2.0,
        low: 0.5,
        close: 1.5,
        volume: -3.0,
    };
    let candle = negative_volume
        .into_candle(interval)
        .expect("finite candle converts");
    assert_eq!(candle.open_time, 1_000);
    assert_eq!(candle.close_time, 1_000 + interval - 1);
    assert_eq!(candle.volume, 0.0);

    let non_finite = SchwabPriceCandle {
        datetime: 1_000,
        open: f64::NAN,
        high: 2.0,
        low: 0.5,
        close: 1.5,
        volume: 1.0,
    };
    assert!(non_finite.into_candle(interval).is_none());
}

#[test]
fn schwab_token_response_supports_missing_optional_fields() {
    let response: SchwabTokenResponse =
        serde_json::from_str(r#"{"access_token":"tok"}"#).expect("token json parses");
    assert_eq!(response.access_token, "tok");
    assert!(response.refresh_token.is_none());
    assert!(response.expires_in.is_none());
}

#[test]
fn schwab_linked_account_response_requires_hash_value() {
    let json = r#"[
        {"accountNumber":"123456789","hashValue":"HASH-1"},
        {"accountNumber":"987654321","hashValue":"   "}
    ]"#;
    let parsed: Vec<SchwabLinkedAccountResponse> =
        serde_json::from_str(json).expect("linked accounts json parses");
    let linked: Vec<_> = parsed
        .into_iter()
        .filter_map(SchwabLinkedAccountResponse::into_linked_account)
        .collect();

    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].hash_value, "HASH-1");
    assert_eq!(linked[0].account_number.as_deref(), Some("123456789"));
}

#[test]
fn schwab_account_envelope_joins_hash_and_parses_positions() {
    let json = r#"[
        {"securitiesAccount":{
            "accountNumber":"123456789",
            "type":"MARGIN",
            "currentBalances":{"cashBalance":250.5,"buyingPower":1000.0,"liquidationValue":1250.5},
            "positions":[
                {"instrument":{"symbol":"AAPL"},"longQuantity":3.0,"shortQuantity":1.0,"marketValue":400.0},
                {"longQuantity":2.0}
            ]
        }},
        {"securitiesAccount":{"accountNumber":"555","type":"CASH"}}
    ]"#;
    let envelopes: Vec<SchwabAccountEnvelope> =
        serde_json::from_str(json).expect("accounts json parses");
    let account_number_to_hash: HashMap<String, String> =
        [("123456789".to_string(), "HASH-1".to_string())].into();

    let summaries: Vec<_> = envelopes
        .into_iter()
        .filter_map(|envelope| envelope.into_summary(&account_number_to_hash))
        .collect();

    // The account without a linked hash is dropped rather than guessed.
    assert_eq!(summaries.len(), 1);
    let summary = &summaries[0];
    assert_eq!(summary.account_hash, "HASH-1");
    assert_eq!(summary.cash_balance, Some(250.5));
    assert_eq!(summary.buying_power, Some(1000.0));
    assert_eq!(summary.liquidation_value, Some(1250.5));
    // The position without an instrument symbol is dropped; quantity nets long/short.
    assert_eq!(summary.positions.len(), 1);
    assert_eq!(summary.positions[0].symbol, "AAPL");
    assert_eq!(summary.positions[0].quantity, 2.0);
    assert_eq!(summary.masked_account_number(), "...6789");
    assert_eq!(summary.label(), "Schwab MARGIN ...6789");
}

#[test]
fn schwab_account_debug_redacts_identity_and_position_values_without_changing_them() {
    let position = SchwabPositionSummary {
        symbol: "private-schwab-symbol-sentinel".to_string(),
        quantity: 12_345.678_9,
        market_value: Some(98_765.432_1),
    };
    let account = SchwabAccountSummary {
        account_number: Some("private-schwab-account-sentinel".to_string()),
        account_hash: "private-schwab-hash-sentinel".to_string(),
        account_type: Some("private-schwab-type-sentinel".to_string()),
        cash_balance: Some(45_678.912_3),
        buying_power: Some(33_333.233_4),
        liquidation_value: Some(22_222.122_3),
        positions: vec![position],
    };
    let snapshot = SchwabAccountsSnapshot {
        linked_accounts: vec![SchwabLinkedAccount {
            account_number: Some("private-linked-account-sentinel".to_string()),
            hash_value: "private-linked-hash-sentinel".to_string(),
        }],
        accounts: vec![account],
    };

    let rendered = format!(
        "{snapshot:?} {:?} {:?}",
        &snapshot.accounts[0], &snapshot.accounts[0].positions[0]
    );

    assert!(rendered.contains("<redacted>"), "{rendered}");
    assert!(rendered.contains("accounts_count: 1"), "{rendered}");
    for sensitive in [
        "private-schwab-symbol-sentinel",
        "private-schwab-account-sentinel",
        "private-schwab-hash-sentinel",
        "private-schwab-type-sentinel",
        "private-linked-account-sentinel",
        "private-linked-hash-sentinel",
        "12345.6789",
        "98765.4321",
        "45678.9123",
    ] {
        assert!(!rendered.contains(sensitive), "{rendered}");
    }
    assert_eq!(
        snapshot.accounts[0].positions[0].symbol,
        "private-schwab-symbol-sentinel"
    );
    assert_eq!(
        snapshot.accounts[0].positions[0].quantity.to_bits(),
        12_345.678_9_f64.to_bits()
    );
    assert_eq!(
        snapshot.accounts[0].account_type.as_deref(),
        Some("private-schwab-type-sentinel")
    );
}

#[test]
fn schwab_state_credential_change_resets_accounts_and_requests() {
    let mut state = SchwabState::new("id", "secret", "access", "refresh");
    state.linked_accounts.push(linked("HASH"));
    state.accounts.push(summary("HASH"));
    state.selected_account_hash = Some("HASH".to_string());
    state.token_refreshing = true;
    state.accounts_loading = true;
    let token_request_id = state.token_refresh_request_id;

    let changed =
        state.set_oauth_credentials_from_secret("id", "secret", "new-access", "refresh", Some(42));

    assert!(changed);
    assert!(state.linked_accounts.is_empty());
    assert!(state.accounts.is_empty());
    assert!(state.selected_account_hash.is_none());
    assert!(!state.token_refreshing);
    assert!(!state.accounts_loading);
    assert!(state.token_refresh_request_id > token_request_id);

    let unchanged =
        state.set_oauth_credentials_from_secret("id", "secret", "new-access", "refresh", Some(43));
    assert!(!unchanged);
}

#[test]
fn schwab_access_token_refresh_due_uses_expiry_and_refresh_credentials() {
    let mut state = SchwabState::new("id", "secret", "access", "refresh");
    // No known expiry: refresh immediately when refresh credentials exist.
    assert!(state.access_token_refresh_due(1_000));

    state.set_oauth_credentials_from_secret("id", "secret", "access2", "refresh", Some(500_000));
    assert!(!state.access_token_refresh_due(400_000));
    assert!(state.access_token_refresh_due(440_001));

    let token_only = SchwabState::new("", "", "access", "");
    assert!(!token_only.access_token_refresh_due(1_000));
}

#[test]
fn schwab_auto_token_refresh_attempts_are_rate_limited() {
    let mut state = SchwabState::new("id", "secret", "access", "refresh");
    assert!(state.auto_token_refresh_attempt_allowed(10_000));

    state.record_auto_token_refresh_attempt(10_000);
    assert!(
        !state.auto_token_refresh_attempt_allowed(10_000 + SCHWAB_AUTO_TOKEN_REFRESH_RETRY_MS - 1)
    );
    assert!(state.auto_token_refresh_attempt_allowed(10_000 + SCHWAB_AUTO_TOKEN_REFRESH_RETRY_MS));

    // New credentials should be allowed an immediate refresh attempt.
    state.set_oauth_credentials_from_secret("id2", "secret", "access", "refresh", None);
    assert!(state.auto_token_refresh_attempt_allowed(10_001));
}

#[test]
fn schwab_accounts_snapshot_selection_prefers_known_account() {
    let mut state = SchwabState::new("", "", "access", "");
    state.selected_account_hash = Some("GONE".to_string());

    state.apply_accounts_snapshot(SchwabAccountsSnapshot {
        linked_accounts: vec![linked("LINK")],
        accounts: vec![summary("KEEP")],
    });
    assert_eq!(state.selected_account_hash.as_deref(), Some("KEEP"));

    // A refresh that still contains the selected account keeps the selection.
    state.apply_accounts_snapshot(SchwabAccountsSnapshot {
        linked_accounts: vec![linked("LINK")],
        accounts: vec![summary("OTHER"), summary("KEEP")],
    });
    assert_eq!(state.selected_account_hash.as_deref(), Some("KEEP"));

    // Without account summaries the selection falls back to a linked account.
    state.apply_accounts_snapshot(SchwabAccountsSnapshot {
        linked_accounts: vec![linked("LINK")],
        accounts: Vec::new(),
    });
    assert_eq!(state.selected_account_hash.as_deref(), Some("LINK"));
    assert_eq!(state.connected_account_count(), 1);
}

#[test]
fn schwab_refresh_credential_input_candidate_requires_all_fields() {
    let mut state = SchwabState::new("", "", "", "");
    state.client_id_input = sensitive_string("id".to_string());
    assert!(state.refresh_credentials_candidate_from_input().is_none());
    assert!(state.status.as_ref().is_some_and(|(_, is_error)| *is_error));

    state.client_id_input = sensitive_string(" id ".to_string());
    state.client_secret_input = sensitive_string("secret".to_string());
    state.refresh_token_input = sensitive_string("refresh".to_string());
    let (client_id, client_secret, refresh_token) = state
        .refresh_credentials_candidate_from_input()
        .expect("complete input yields candidate");

    assert_eq!(client_id.as_str(), "id");
    assert_eq!(client_secret.as_str(), "secret");
    assert_eq!(refresh_token.as_str(), "refresh");
    // Inputs are zeroized once captured; the pending copy backs the secret save.
    assert!(state.client_id_input.trim().is_empty());
    assert!(state.client_secret_input.trim().is_empty());
    assert!(state.refresh_token_input.trim().is_empty());
    let (pending_id, _, _) = state
        .pending_refresh_credentials_for_secret()
        .expect("pending credentials retained");
    assert_eq!(pending_id.as_str(), "id");
}

#[test]
fn schwab_state_debug_redacts_credentials() {
    let state = SchwabState::new(
        "id-material",
        "secret-material",
        "access-material",
        "refresh-material",
    );
    let rendered = format!("{state:?}");

    assert!(rendered.contains("<redacted>"));
    for secret in [
        "id-material",
        "secret-material",
        "access-material",
        "refresh-material",
    ] {
        assert!(!rendered.contains(secret), "debug output leaked {secret}");
    }
}

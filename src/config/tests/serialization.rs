use super::super::secrets;
use super::super::{
    AccountProfile, ChartConfig, CredentialStorageMode, EncryptedSecretsConfig, KeroseneConfig,
    MacroIndicatorsConfig, PaneKindConfig, PaneLayoutConfig, default_market_slippage_pct,
};
use crate::advanced_order_history::{AdvancedOrderHistoryEntry, AdvancedOrderHistoryKind};
use std::collections::HashMap;

#[test]
fn legacy_journal_entries_deserialize_without_account_scope() {
    let mut value =
        serde_json::to_value(KeroseneConfig::default()).expect("default config should serialize");
    let object = value
        .as_object_mut()
        .expect("config should serialize to object");
    object.remove("journal_entries_by_account");
    object.insert(
        "journal_entries".to_string(),
        serde_json::json!({
            "BTC_1": {
                "open": "legacy note",
                "close": ""
            }
        }),
    );

    let config: KeroseneConfig =
        serde_json::from_value(value).expect("legacy journal config should deserialize");

    assert!(config.journal_entries_by_account.is_empty());
    assert_eq!(
        config
            .journal_entries
            .get("BTC_1")
            .map(|entry| entry.open.as_str()),
        Some("legacy note")
    );
}

#[test]
fn serialized_config_keeps_raw_credentials_out_of_json() {
    let profiles = vec![AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Main".to_string(),
        wallet_address: String::new(),
        agent_key: "agent-secret".to_string().into(),
        hydromancer_api_key: String::new().into(),
    }];
    let config = KeroseneConfig {
        credential_storage_mode: CredentialStorageMode::EncryptedConfig,
        encrypted_secrets: Some(EncryptedSecretsConfig {
            version: 1,
            kdf: secrets::SecretKdfConfig {
                algorithm: "argon2id".to_string(),
                salt: "test-salt".to_string(),
                memory_kib: 64,
                iterations: 1,
                lanes: 1,
            },
            cipher: "xchacha20poly1305".to_string(),
            nonce: "test-nonce".to_string(),
            ciphertext: "encrypted payload".to_string(),
        }),
        accounts: profiles,
        agent_key: "legacy-agent-secret".to_string().into(),
        hydromancer_api_key: "hydro-secret".to_string().into(),
        hyperdash_api_key: "hyper-secret".to_string().into(),
        ..KeroseneConfig::default()
    };

    let json = serde_json::to_string(&config).expect("config should serialize");

    assert!(json.contains("encrypted_secrets"));
    assert!(!json.contains("agent-secret"));
    assert!(!json.contains("legacy-agent-secret"));
    assert!(!json.contains("hydro-secret"));
    assert!(!json.contains("hyper-secret"));
}

#[test]
fn legacy_config_without_market_slippage_uses_default() {
    let mut value =
        serde_json::to_value(KeroseneConfig::default()).expect("default config should serialize");
    value
        .as_object_mut()
        .expect("config should serialize to object")
        .remove("market_slippage_pct");

    let config: KeroseneConfig =
        serde_json::from_value(value).expect("legacy config should deserialize");

    assert_eq!(config.market_slippage_pct, default_market_slippage_pct());
}

#[test]
fn symbol_search_sort_mode_round_trips_and_legacy_defaults_relevance() {
    let config = KeroseneConfig {
        symbol_search_sort_mode: "24h_volume".to_string(),
        ..KeroseneConfig::default()
    };

    let json = serde_json::to_string(&config).expect("config should serialize");
    let decoded: KeroseneConfig = serde_json::from_str(&json).expect("config should deserialize");
    assert_eq!(decoded.symbol_search_sort_mode, "24h_volume");

    let mut legacy =
        serde_json::to_value(KeroseneConfig::default()).expect("default config should serialize");
    legacy
        .as_object_mut()
        .expect("config should serialize to object")
        .remove("symbol_search_sort_mode");
    let decoded_legacy: KeroseneConfig =
        serde_json::from_value(legacy).expect("legacy config should deserialize");
    assert_eq!(decoded_legacy.symbol_search_sort_mode, "relevance");
}

#[test]
fn hide_pnl_round_trips_and_legacy_defaults_visible() {
    let config = KeroseneConfig {
        hide_pnl: true,
        ..KeroseneConfig::default()
    };

    let json = serde_json::to_string(&config).expect("config should serialize");
    let decoded: KeroseneConfig = serde_json::from_str(&json).expect("config should deserialize");
    assert!(decoded.hide_pnl);

    let mut legacy =
        serde_json::to_value(KeroseneConfig::default()).expect("default config should serialize");
    legacy
        .as_object_mut()
        .expect("config should serialize to object")
        .remove("hide_pnl");
    let decoded_legacy: KeroseneConfig =
        serde_json::from_value(legacy).expect("legacy config should deserialize");
    assert!(!decoded_legacy.hide_pnl);
}

#[test]
fn hidden_positions_round_trip_and_legacy_defaults_empty() {
    let config = KeroseneConfig {
        hidden_positions_by_account: HashMap::from([(
            "acct-a".to_string(),
            vec!["BTC".to_string(), "xyz:CRCL".to_string()],
        )]),
        ..KeroseneConfig::default()
    };

    let json = serde_json::to_string(&config).expect("config should serialize");
    let decoded: KeroseneConfig = serde_json::from_str(&json).expect("config should deserialize");
    assert_eq!(
        decoded.hidden_positions_by_account.get("acct-a"),
        Some(&vec!["BTC".to_string(), "xyz:CRCL".to_string()])
    );

    let mut legacy =
        serde_json::to_value(KeroseneConfig::default()).expect("default config should serialize");
    legacy
        .as_object_mut()
        .expect("config should serialize to object")
        .remove("hidden_positions_by_account");
    let decoded_legacy: KeroseneConfig =
        serde_json::from_value(legacy).expect("legacy config should deserialize");
    assert!(decoded_legacy.hidden_positions_by_account.is_empty());
}

#[test]
fn advanced_order_history_round_trips_and_legacy_defaults_empty() {
    let config = KeroseneConfig {
        advanced_order_history: vec![AdvancedOrderHistoryEntry {
            id: "twap:acct:1000:1".to_string(),
            kind: AdvancedOrderHistoryKind::Twap,
            source_id: 1,
            account_address: "0xabc".to_string(),
            coin: "BTC".to_string(),
            display_coin: "BTC".to_string(),
            is_buy: true,
            target_size: 1.0,
            filled_size: 1.0,
            remaining_size: 0.0,
            average_price: Some(100.0),
            min_price: Some(99.0),
            max_price: Some(101.0),
            reduce_only: false,
            randomize: true,
            slice_count: 2,
            slices_sent: 2,
            reprice_count: 0,
            status: "Completed".to_string(),
            summary: "TWAP completed".to_string(),
            started_at_ms: 1_000,
            completed_at_ms: 2_000,
            logs: Vec::new(),
            children: Vec::new(),
        }],
        ..KeroseneConfig::default()
    };

    let json = serde_json::to_string(&config).expect("config should serialize");
    let decoded: KeroseneConfig = serde_json::from_str(&json).expect("config should deserialize");
    assert_eq!(decoded.advanced_order_history.len(), 1);
    assert_eq!(decoded.advanced_order_history[0].status, "Completed");

    let mut legacy =
        serde_json::to_value(KeroseneConfig::default()).expect("default config should serialize");
    legacy
        .as_object_mut()
        .expect("config should serialize to object")
        .remove("advanced_order_history");
    let decoded_legacy: KeroseneConfig =
        serde_json::from_value(legacy).expect("legacy config should deserialize");
    assert!(decoded_legacy.advanced_order_history.is_empty());
}

#[test]
fn chart_trade_marker_toggle_round_trips_and_legacy_defaults_off() {
    let config = KeroseneConfig {
        charts: vec![ChartConfig {
            id: 7,
            symbol: "BTC".to_string(),
            timeframe: "H1".to_string(),
            annotations: Vec::new(),
            inverted: false,
            show_trade_markers: true,
            funding_panel_height: 56,
            macro_indicators: MacroIndicatorsConfig::default(),
            open_interest_as_notional: true,
        }],
        ..KeroseneConfig::default()
    };

    let json = serde_json::to_string(&config).expect("config should serialize");
    let decoded: KeroseneConfig = serde_json::from_str(&json).expect("config should deserialize");

    assert!(decoded.charts[0].show_trade_markers);
    assert!(decoded.charts[0].open_interest_as_notional);

    let mut legacy_chart = serde_json::to_value(&config.charts[0]).expect("chart serializes");
    legacy_chart
        .as_object_mut()
        .expect("chart config is an object")
        .remove("show_trade_markers");
    let decoded_chart: ChartConfig =
        serde_json::from_value(legacy_chart).expect("legacy chart config should deserialize");

    assert!(!decoded_chart.show_trade_markers);
    assert!(decoded_chart.open_interest_as_notional);

    let mut older_chart = serde_json::to_value(&config.charts[0]).expect("chart serializes");
    older_chart
        .as_object_mut()
        .expect("chart config is an object")
        .remove("open_interest_as_notional");
    let decoded_older_chart: ChartConfig =
        serde_json::from_value(older_chart).expect("older chart config should deserialize");

    assert!(!decoded_older_chart.open_interest_as_notional);
}

#[test]
fn legacy_assistant_pane_deserializes_as_unsupported() {
    let layout: PaneLayoutConfig = serde_json::from_value(serde_json::json!({"Leaf": "Assistant"}))
        .expect("legacy assistant pane should deserialize");

    assert_eq!(layout, PaneLayoutConfig::Leaf(PaneKindConfig::Unsupported));
}

#[test]
fn serialized_config_omits_removed_assistant_settings() {
    let json =
        serde_json::to_string(&KeroseneConfig::default()).expect("default config should serialize");

    assert!(!json.contains("assistant_api_key"));
    assert!(!json.contains("assistant_model"));
    assert!(!json.contains("assistant_use_account_context"));
    assert!(!json.contains("assistant_allow_code_execution"));
}

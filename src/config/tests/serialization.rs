use super::super::secrets;
use super::super::{
    AccountProfile, ChartConfig, ChartScreenshotSettingsConfig, CredentialStorageMode,
    CustomFontConfig, DetachedChartWindowConfig, DisplayDenominationConfig, DisplayFontConfig,
    EncryptedSecretsConfig, KeroseneConfig, MacroIndicatorsConfig, PaneKindConfig,
    PaneLayoutConfig, default_alfred_popup_scale, default_market_slippage_pct,
    default_pane_border_thickness, default_pane_corner_radius, default_ui_scale,
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
fn display_denomination_round_trips_and_legacy_defaults_usd() {
    let config = KeroseneConfig {
        display_denomination: DisplayDenominationConfig::eur(),
        ..KeroseneConfig::default()
    };

    let json = serde_json::to_string(&config).expect("config should serialize");
    let decoded: KeroseneConfig = serde_json::from_str(&json).expect("config should deserialize");
    assert_eq!(
        decoded.display_denomination,
        DisplayDenominationConfig::eur()
    );

    let hype_config = KeroseneConfig {
        display_denomination: DisplayDenominationConfig::hype(),
        ..KeroseneConfig::default()
    };
    let hype_json = serde_json::to_string(&hype_config).expect("config should serialize");
    let decoded_hype: KeroseneConfig =
        serde_json::from_str(&hype_json).expect("config should deserialize");
    assert_eq!(
        decoded_hype.display_denomination,
        DisplayDenominationConfig::hype()
    );

    let btc_config = KeroseneConfig {
        display_denomination: DisplayDenominationConfig::btc(),
        ..KeroseneConfig::default()
    };
    let btc_json = serde_json::to_string(&btc_config).expect("config should serialize");
    let decoded_btc: KeroseneConfig =
        serde_json::from_str(&btc_json).expect("config should deserialize");
    assert_eq!(
        decoded_btc.display_denomination,
        DisplayDenominationConfig::btc()
    );

    let mut legacy =
        serde_json::to_value(KeroseneConfig::default()).expect("default config should serialize");
    legacy
        .as_object_mut()
        .expect("config should serialize to object")
        .remove("display_denomination");
    let decoded_legacy: KeroseneConfig =
        serde_json::from_value(legacy).expect("legacy config should deserialize");
    assert_eq!(
        decoded_legacy.display_denomination,
        DisplayDenominationConfig::Usd
    );
}

#[test]
fn widget_chrome_round_trips_and_legacy_defaults_current_values() {
    let config = KeroseneConfig {
        ui_scale: 0.85,
        alfred_popup_scale: 1.35,
        pane_border_thickness: 8.0,
        pane_corner_radius: 12.0,
        ..KeroseneConfig::default()
    };

    let json = serde_json::to_string(&config).expect("config should serialize");
    let decoded: KeroseneConfig = serde_json::from_str(&json).expect("config should deserialize");
    assert_eq!(decoded.ui_scale, 0.85);
    assert_eq!(decoded.alfred_popup_scale, 1.35);
    assert_eq!(decoded.pane_border_thickness, 8.0);
    assert_eq!(decoded.pane_corner_radius, 12.0);

    let mut legacy =
        serde_json::to_value(KeroseneConfig::default()).expect("default config should serialize");
    let object = legacy
        .as_object_mut()
        .expect("config should serialize to object");
    object.remove("ui_scale");
    object.remove("alfred_popup_scale");
    object.remove("pane_border_thickness");
    object.remove("pane_corner_radius");

    let decoded_legacy: KeroseneConfig =
        serde_json::from_value(legacy).expect("legacy config should deserialize");
    assert_eq!(decoded_legacy.ui_scale, default_ui_scale());
    assert_eq!(
        decoded_legacy.alfred_popup_scale,
        default_alfred_popup_scale()
    );
    assert_eq!(
        decoded_legacy.pane_border_thickness,
        default_pane_border_thickness()
    );
    assert_eq!(
        decoded_legacy.pane_corner_radius,
        default_pane_corner_radius()
    );
}

#[test]
fn display_and_monospace_fonts_round_trip_and_legacy_default_system() {
    let config = KeroseneConfig {
        display_font: DisplayFontConfig::Custom {
            family: "Inter".to_string(),
        },
        monospace_font: DisplayFontConfig::Custom {
            family: "Roboto Mono".to_string(),
        },
        custom_fonts: vec![CustomFontConfig {
            family: "Inter".to_string(),
            file_name: "inter.ttf".to_string(),
        }],
        ..KeroseneConfig::default()
    };

    let json = serde_json::to_string(&config).expect("config should serialize");
    let decoded: KeroseneConfig = serde_json::from_str(&json).expect("config should deserialize");
    assert_eq!(decoded.display_font, config.display_font);
    assert_eq!(decoded.monospace_font, config.monospace_font);
    assert_eq!(decoded.custom_fonts, config.custom_fonts);

    let mut legacy =
        serde_json::to_value(KeroseneConfig::default()).expect("default config should serialize");
    let object = legacy
        .as_object_mut()
        .expect("config should serialize to object");
    object.remove("display_font");
    object.remove("monospace_font");
    object.remove("custom_fonts");

    let decoded_legacy: KeroseneConfig =
        serde_json::from_value(legacy).expect("legacy config should deserialize");
    assert_eq!(decoded_legacy.display_font, DisplayFontConfig::System);
    assert_eq!(decoded_legacy.monospace_font, DisplayFontConfig::System);
    assert!(decoded_legacy.custom_fonts.is_empty());
}

#[test]
fn bundled_display_and_monospace_fonts_do_not_require_custom_font_entries() {
    for family in crate::config::BUNDLED_DISPLAY_FONT_FAMILIES {
        let config = KeroseneConfig {
            display_font: DisplayFontConfig::Custom {
                family: family.to_ascii_lowercase(),
            },
            monospace_font: DisplayFontConfig::Custom {
                family: family.to_ascii_lowercase(),
            },
            custom_fonts: Vec::new(),
            ..KeroseneConfig::default()
        };
        let custom_fonts = crate::config::normalize_custom_fonts(config.custom_fonts);
        let display_font =
            crate::config::normalize_display_font(config.display_font, &custom_fonts);
        let monospace_font =
            crate::config::normalize_display_font(config.monospace_font, &custom_fonts);

        assert_eq!(
            display_font,
            DisplayFontConfig::Custom {
                family: (*family).to_string()
            }
        );
        assert_eq!(
            monospace_font,
            DisplayFontConfig::Custom {
                family: (*family).to_string()
            }
        );
    }
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
fn chart_timeframe_hotkey_prefix_round_trips_and_legacy_defaults_none() {
    let config = KeroseneConfig {
        chart_timeframe_hotkey_prefix: Some(crate::config::HotkeyPrefixConfig {
            shift: false,
            ctrl: false,
            alt: false,
            logo: true,
        }),
        ..KeroseneConfig::default()
    };

    let json = serde_json::to_string(&config).expect("config should serialize");
    let decoded: KeroseneConfig = serde_json::from_str(&json).expect("config should deserialize");
    assert_eq!(
        decoded.chart_timeframe_hotkey_prefix,
        config.chart_timeframe_hotkey_prefix
    );

    let mut legacy =
        serde_json::to_value(KeroseneConfig::default()).expect("default config should serialize");
    legacy
        .as_object_mut()
        .expect("config should serialize to object")
        .remove("chart_timeframe_hotkey_prefix");
    let decoded_legacy: KeroseneConfig =
        serde_json::from_value(legacy).expect("legacy config should deserialize");
    assert_eq!(decoded_legacy.chart_timeframe_hotkey_prefix, None);
}

#[test]
fn chart_screenshot_settings_round_trip_and_legacy_defaults_visible() {
    let config = KeroseneConfig {
        chart_screenshot_settings: ChartScreenshotSettingsConfig {
            obscure_position_entry: true,
            hide_positions_and_orders: true,
        },
        ..KeroseneConfig::default()
    };

    let json = serde_json::to_string(&config).expect("config should serialize");
    let decoded: KeroseneConfig = serde_json::from_str(&json).expect("config should deserialize");
    assert!(decoded.chart_screenshot_settings.obscure_position_entry);
    assert!(decoded.chart_screenshot_settings.hide_positions_and_orders);

    let mut legacy =
        serde_json::to_value(KeroseneConfig::default()).expect("default config should serialize");
    legacy
        .as_object_mut()
        .expect("config should serialize to object")
        .remove("chart_screenshot_settings");
    let decoded_legacy: KeroseneConfig =
        serde_json::from_value(legacy).expect("legacy config should deserialize");
    assert!(
        !decoded_legacy
            .chart_screenshot_settings
            .obscure_position_entry
    );
    assert!(
        !decoded_legacy
            .chart_screenshot_settings
            .hide_positions_and_orders
    );
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
fn order_quantity_denomination_round_trips_and_legacy_defaults_coin() {
    let config = KeroseneConfig {
        order_quantity_is_usd: true,
        ..KeroseneConfig::default()
    };

    let json = serde_json::to_string(&config).expect("config should serialize");
    let decoded: KeroseneConfig = serde_json::from_str(&json).expect("config should deserialize");
    assert!(decoded.order_quantity_is_usd);

    let mut legacy =
        serde_json::to_value(KeroseneConfig::default()).expect("default config should serialize");
    legacy
        .as_object_mut()
        .expect("config should serialize to object")
        .remove("order_quantity_is_usd");
    let decoded_legacy: KeroseneConfig =
        serde_json::from_value(legacy).expect("legacy config should deserialize");
    assert!(!decoded_legacy.order_quantity_is_usd);
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
    let macro_indicators = MacroIndicatorsConfig {
        show_volume_profile: true,
        ..MacroIndicatorsConfig::default()
    };
    let config = KeroseneConfig {
        charts: vec![ChartConfig {
            id: 7,
            symbol: "BTC".to_string(),
            timeframe: "H1".to_string(),
            annotations: Vec::new(),
            inverted: false,
            show_trade_markers: true,
            funding_panel_height: 56,
            macro_indicators,
            open_interest_as_notional: true,
        }],
        ..KeroseneConfig::default()
    };

    let json = serde_json::to_string(&config).expect("config should serialize");
    let decoded: KeroseneConfig = serde_json::from_str(&json).expect("config should deserialize");

    assert!(decoded.charts[0].show_trade_markers);
    assert!(decoded.charts[0].open_interest_as_notional);
    assert!(decoded.charts[0].macro_indicators.show_volume_profile);

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

    let mut legacy_macro = serde_json::to_value(&config.charts[0].macro_indicators)
        .expect("macro indicators serialize");
    legacy_macro
        .as_object_mut()
        .expect("macro indicators config is an object")
        .remove("show_volume_profile");
    let decoded_macro: MacroIndicatorsConfig =
        serde_json::from_value(legacy_macro).expect("legacy macro indicators should deserialize");

    assert!(!decoded_macro.show_volume_profile);
}

#[test]
fn detached_chart_windows_round_trip_and_legacy_defaults_empty() {
    let config = KeroseneConfig {
        detached_chart_windows: vec![DetachedChartWindowConfig {
            chart_id: 7,
            width: 1200.0,
            height: 760.0,
            x: Some(1800.0),
            y: Some(40.0),
        }],
        ..KeroseneConfig::default()
    };

    let json = serde_json::to_string(&config).expect("config should serialize");
    let decoded: KeroseneConfig = serde_json::from_str(&json).expect("config should deserialize");

    assert_eq!(decoded.detached_chart_windows.len(), 1);
    assert_eq!(decoded.detached_chart_windows[0].chart_id, 7);
    assert_eq!(decoded.detached_chart_windows[0].width, 1200.0);
    assert_eq!(decoded.detached_chart_windows[0].x, Some(1800.0));

    let mut legacy =
        serde_json::to_value(KeroseneConfig::default()).expect("default config should serialize");
    legacy
        .as_object_mut()
        .expect("config should serialize to object")
        .remove("detached_chart_windows");
    let decoded_legacy: KeroseneConfig =
        serde_json::from_value(legacy).expect("legacy config should deserialize");

    assert!(decoded_legacy.detached_chart_windows.is_empty());
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

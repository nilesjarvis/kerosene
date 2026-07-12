use super::*;

fn profile(secret_id: &str) -> AccountProfile {
    AccountProfile {
        secret_id: secret_id.to_string(),
        name: secret_id.to_string(),
        wallet_address: String::new(),
        agent_key: String::new().into(),
        hydromancer_api_key: String::new().into(),
    }
}

fn bloomberg_theme(
    chart_line: Option<&str>,
    chart_line_gradient: Option<&str>,
) -> crate::config::CustomThemeConfig {
    crate::config::CustomThemeConfig {
        name: "Bloomberg".to_string(),
        background: "#000000".to_string(),
        text: "#F2F2E8".to_string(),
        primary: "#FF9F1A".to_string(),
        success: "#00B050".to_string(),
        warning: "#FFD84A".to_string(),
        danger: "#B00024".to_string(),
        chart_bull: Some("#00C853".to_string()),
        chart_bear: Some("#D50032".to_string()),
        chart_line: chart_line.map(str::to_string),
        chart_line_gradient: chart_line_gradient.map(str::to_string),
    }
}

#[test]
fn normalizes_bloomberg_chart_line_override() {
    let mut config = KeroseneConfig {
        custom_themes: vec![bloomberg_theme(None, None)],
        ..KeroseneConfig::default()
    };

    normalize_loaded_config(&mut config);

    let bloomberg = config
        .custom_themes
        .iter()
        .find(|theme| theme.name == "Bloomberg")
        .expect("Bloomberg theme should be present");
    assert_eq!(bloomberg.chart_line.as_deref(), Some("#9AD7FF"));
    assert_eq!(bloomberg.chart_line_gradient.as_deref(), Some("#0054A6"));
}

#[test]
fn normalizes_previous_bloomberg_chart_line_default() {
    let mut config = KeroseneConfig {
        custom_themes: vec![bloomberg_theme(Some("#0054A6"), None)],
        ..KeroseneConfig::default()
    };

    normalize_loaded_config(&mut config);

    let bloomberg = config
        .custom_themes
        .iter()
        .find(|theme| theme.name == "Bloomberg")
        .expect("Bloomberg theme should be present");
    assert_eq!(bloomberg.chart_line.as_deref(), Some("#9AD7FF"));
    assert_eq!(bloomberg.chart_line_gradient.as_deref(), Some("#0054A6"));
}

#[test]
fn normalizes_missing_chart_line_without_overwriting_custom_value() {
    let mut config = KeroseneConfig {
        custom_themes: vec![bloomberg_theme(Some("#123456"), None)],
        ..KeroseneConfig::default()
    };

    normalize_loaded_config(&mut config);

    let bloomberg = config
        .custom_themes
        .iter()
        .find(|theme| theme.name == "Bloomberg")
        .expect("Bloomberg theme should be present");
    assert_eq!(bloomberg.chart_line.as_deref(), Some("#123456"));
    assert_eq!(bloomberg.chart_line_gradient.as_deref(), Some("#0054A6"));
}

#[test]
fn adds_cursor_theme_to_existing_config() {
    let mut config = KeroseneConfig {
        custom_themes: Vec::new(),
        ..KeroseneConfig::default()
    };

    normalize_loaded_config(&mut config);

    let cursor = config
        .custom_themes
        .iter()
        .find(|theme| theme.name == "Cursor")
        .expect("Cursor theme should be present");
    assert_eq!(cursor.background, "#14120B");
    assert_eq!(cursor.text, "#EDECEC");
    assert_eq!(cursor.primary, "#F54E00");
}

#[test]
fn normalizes_out_of_range_market_slippage() {
    let mut value =
        serde_json::to_value(KeroseneConfig::default()).expect("default config serializes");
    let object = value
        .as_object_mut()
        .expect("config should serialize to object");
    object.insert("market_slippage_pct".to_string(), serde_json::json!(99.0));
    object.insert(
        "saved_layouts".to_string(),
        serde_json::json!([
            {
                "name": "bad-slippage",
                "market_slippage_pct": 99.0,
            }
        ]),
    );
    let mut config: KeroseneConfig =
        serde_json::from_value(value).expect("test config deserializes");

    normalize_loaded_config(&mut config);

    assert_eq!(config.market_slippage_pct, default_market_slippage_pct());
    assert_eq!(
        config.saved_layouts[0].market_slippage_pct,
        default_market_slippage_pct()
    );
}

#[test]
fn normalizes_out_of_range_pane_chrome() {
    let mut config = KeroseneConfig {
        ui_scale: 99.0,
        alfred_popup_scale: 99.0,
        chart_dotted_background_opacity: 99.0,
        chart_gradient_contrast: 99.0,
        chart_fisheye_strength: 99.0,
        chart_chromatic_aberration_strength: 99.0,
        chart_edge_blur_strength: 99.0,
        pane_border_thickness: 99.0,
        pane_corner_radius: f32::NAN,
        widget_padding: crate::config::WidgetPaddingConfig {
            default_px: 99.0,
            overrides: vec![
                crate::config::WidgetPaddingOverrideConfig {
                    target: crate::config::WidgetPaddingTargetConfig::Watchlist,
                    padding_px: 12.0,
                },
                crate::config::WidgetPaddingOverrideConfig {
                    target: crate::config::WidgetPaddingTargetConfig::Watchlist,
                    padding_px: 99.0,
                },
            ],
        },
        saved_layouts: vec![
            serde_json::from_value(serde_json::json!({
                "name": "bad-padding",
                "widget_padding": {
                    "default_px": -10.0,
                    "overrides": [
                        {
                            "target": "OrderEntry",
                            "padding_px": 99.0
                        }
                    ]
                }
            }))
            .expect("saved layout should deserialize"),
        ],
        ..KeroseneConfig::default()
    };

    normalize_loaded_config(&mut config);

    assert_eq!(config.ui_scale, normalize_ui_scale(99.0));
    assert_eq!(
        config.alfred_popup_scale,
        normalize_alfred_popup_scale(99.0)
    );
    assert_eq!(
        config.chart_dotted_background_opacity,
        crate::config::normalize_chart_dotted_background_opacity(99.0)
    );
    assert_eq!(
        config.chart_gradient_contrast,
        crate::config::normalize_chart_gradient_contrast(99.0)
    );
    assert_eq!(
        config.chart_fisheye_strength,
        crate::config::normalize_chart_fisheye_strength(99.0)
    );
    assert_eq!(
        config.chart_chromatic_aberration_strength,
        crate::config::normalize_chart_chromatic_aberration_strength(99.0)
    );
    assert_eq!(
        config.chart_edge_blur_strength,
        crate::config::normalize_chart_edge_blur_strength(99.0)
    );
    assert_eq!(
        config.pane_border_thickness,
        normalize_pane_border_thickness(99.0)
    );
    assert_eq!(
        config.pane_corner_radius,
        crate::config::default_pane_corner_radius()
    );
    assert_eq!(
        config.widget_padding.default_px,
        crate::config::MAX_WIDGET_PADDING
    );
    assert!(config.widget_padding.overrides.is_empty());
    assert_eq!(config.saved_layouts[0].widget_padding.default_px, 0.0);
    assert_eq!(
        config.saved_layouts[0].widget_padding.overrides[0].padding_px,
        crate::config::MAX_WIDGET_PADDING
    );
}

#[test]
fn normalize_loaded_config_keeps_known_widget_padding_after_unknown_targets_are_dropped() {
    let mut value =
        serde_json::to_value(KeroseneConfig::default()).expect("default config serializes");
    let object = value
        .as_object_mut()
        .expect("config should serialize to object");
    object.insert(
        "widget_padding".to_string(),
        serde_json::json!({
            "default_px": 5.0,
            "overrides": [
                {
                    "target": "Watchlist",
                    "padding_px": 99.0
                },
                {
                    "target": {
                        "FuturePane": {
                            "id": 7
                        }
                    },
                    "padding_px": 12.0
                }
            ]
        }),
    );
    let mut config: KeroseneConfig =
        serde_json::from_value(value).expect("unknown widget padding targets should be dropped");

    assert_eq!(config.widget_padding.overrides.len(), 1);

    normalize_loaded_config(&mut config);

    assert_eq!(config.widget_padding.default_px, 5.0);
    assert_eq!(config.widget_padding.overrides.len(), 1);
    assert_eq!(
        config.widget_padding.overrides[0].target,
        crate::config::WidgetPaddingTargetConfig::Watchlist
    );
    assert_eq!(
        config.widget_padding.overrides[0].padding_px,
        crate::config::MAX_WIDGET_PADDING
    );
}

#[test]
fn migrates_legacy_hollow_candle_toggle_to_up_candles() {
    let mut value =
        serde_json::to_value(KeroseneConfig::default()).expect("default config serializes");
    let object = value
        .as_object_mut()
        .expect("config should serialize to object");
    object.insert("chart_hollow_candles".to_string(), serde_json::json!(true));
    object.remove("chart_hollow_candle_mode");
    let mut config: KeroseneConfig =
        serde_json::from_value(value).expect("test config deserializes");

    normalize_loaded_config(&mut config);

    assert_eq!(
        config.chart_hollow_candle_mode,
        crate::config::ChartHollowCandleMode::Up
    );
    assert!(!config.chart_hollow_candles);
}

#[test]
fn migrates_legacy_chart_backfill_source_to_read_data_provider() {
    let mut value =
        serde_json::to_value(KeroseneConfig::default()).expect("default config serializes");
    let object = value
        .as_object_mut()
        .expect("config should serialize to object");
    object.remove("read_data_provider");
    object.insert(
        "chart_backfill_source".to_string(),
        serde_json::json!("Hydromancer"),
    );
    let mut config: KeroseneConfig =
        serde_json::from_value(value).expect("test config deserializes");

    normalize_loaded_config(&mut config);

    assert_eq!(
        config.read_data_provider,
        crate::config::ReadDataProvider::Hydromancer
    );
    assert_eq!(
        config.chart_backfill_source,
        crate::config::ChartBackfillSource::Hydromancer
    );
}

#[test]
fn migrates_leftover_legacy_agent_key_into_active_account() {
    let mut config = KeroseneConfig {
        active_account_index: 1,
        agent_key: "legacy-active-agent".to_string().into(),
        accounts: vec![
            AccountProfile {
                secret_id: "one".to_string(),
                name: "one".to_string(),
                wallet_address: "0x1111111111111111111111111111111111111111".to_string(),
                agent_key: String::new().into(),
                hydromancer_api_key: String::new().into(),
            },
            AccountProfile {
                secret_id: "two".to_string(),
                name: "two".to_string(),
                wallet_address: "0x2222222222222222222222222222222222222222".to_string(),
                agent_key: String::new().into(),
                hydromancer_api_key: String::new().into(),
            },
        ],
        ..KeroseneConfig::default()
    };

    normalize_loaded_config(&mut config);

    assert_eq!(config.accounts[0].agent_key.as_str(), "");
    assert_eq!(config.accounts[1].agent_key.as_str(), "legacy-active-agent");
    assert_eq!(config.agent_key.as_str(), "");
}

#[test]
fn leftover_legacy_agent_key_does_not_overwrite_existing_active_account_key() {
    let mut config = KeroseneConfig {
        active_account_index: 0,
        agent_key: "legacy-agent".to_string().into(),
        accounts: vec![AccountProfile {
            secret_id: "one".to_string(),
            name: "one".to_string(),
            wallet_address: "0x1111111111111111111111111111111111111111".to_string(),
            agent_key: "account-agent".to_string().into(),
            hydromancer_api_key: String::new().into(),
        }],
        ..KeroseneConfig::default()
    };

    normalize_loaded_config(&mut config);

    assert_eq!(config.accounts[0].agent_key.as_str(), "account-agent");
    assert_eq!(config.agent_key.as_str(), "");
}

#[test]
fn duplicate_and_blank_account_secret_ids_are_repaired_before_pending_keychain_delete() {
    let mut config = KeroseneConfig {
        active_account_index: 1,
        accounts: vec![
            profile("account-a"),
            profile("account-a"),
            profile(" "),
            profile(" account-d "),
            profile(" account-a "),
        ],
        pending_keychain_profile_deletions: vec!["account-a".to_string(), "account-z".to_string()],
        hidden_positions_by_account: std::collections::HashMap::from([
            ("account-a".to_string(), vec!["BTC".to_string()]),
            (" account-d ".to_string(), vec!["ETH".to_string()]),
            (" account-a ".to_string(), vec!["SOL".to_string()]),
        ]),
        journal_entries_by_account: std::collections::HashMap::from([
            ("account-a".to_string(), std::collections::HashMap::new()),
            (" account-d ".to_string(), std::collections::HashMap::new()),
            (" account-a ".to_string(), std::collections::HashMap::new()),
        ]),
        ..KeroseneConfig::default()
    };

    normalize_loaded_config(&mut config);

    assert!(config.secret_cleanup_state_dirty);
    let ids: Vec<&str> = config
        .accounts
        .iter()
        .map(|profile| profile.secret_id.as_str())
        .collect();
    let unique_ids: std::collections::HashSet<&str> = ids.iter().copied().collect();
    assert_eq!(ids.len(), 5);
    assert_eq!(unique_ids.len(), ids.len());
    assert_eq!(ids[0], "account-a");
    assert_ne!(ids[1], "account-a");
    assert!(ids[1].starts_with("acct-"));
    assert!(ids[2].starts_with("acct-"));
    assert_eq!(ids[3], "account-d");
    assert!(ids[4].starts_with("acct-"));
    assert_eq!(config.active_account_index, 1);

    assert_eq!(
        config.pending_keychain_profile_deletions.as_slice(),
        ["account-z"]
    );
    assert!(config.hidden_positions_by_account.contains_key("account-a"));
    assert!(config.hidden_positions_by_account.contains_key("account-d"));
    assert!(config.hidden_positions_by_account.contains_key(ids[4]));
    assert!(
        !config
            .hidden_positions_by_account
            .contains_key(" account-d ")
    );
    assert!(
        !config
            .hidden_positions_by_account
            .contains_key(" account-a ")
    );
    assert!(config.journal_entries_by_account.contains_key("account-a"));
    assert!(config.journal_entries_by_account.contains_key("account-d"));
    assert!(config.journal_entries_by_account.contains_key(ids[4]));
    assert!(
        !config
            .journal_entries_by_account
            .contains_key(" account-d ")
    );
    assert!(
        !config
            .journal_entries_by_account
            .contains_key(" account-a ")
    );
}

#[test]
fn pending_keychain_delete_removes_account_and_clamps_active_index() {
    let mut config = KeroseneConfig {
        active_account_index: 1,
        accounts: vec![
            profile("account-a"),
            profile("account-b"),
            profile("account-c"),
        ],
        pending_keychain_profile_deletions: vec![" account-b ".to_string()],
        ..KeroseneConfig::default()
    };

    normalize_loaded_config(&mut config);

    assert!(config.secret_cleanup_state_dirty);
    assert_eq!(
        config
            .accounts
            .iter()
            .map(|profile| profile.secret_id.as_str())
            .collect::<Vec<_>>(),
        ["account-a", "account-c"]
    );
    assert_eq!(config.active_account_index, 0);
    assert_eq!(
        config.pending_keychain_profile_deletions.as_slice(),
        ["account-b"]
    );
}

#[test]
fn pending_keychain_delete_normalization_marks_cleanup_state_dirty() {
    let mut config = KeroseneConfig {
        pending_keychain_profile_deletions: vec![
            " account-b ".to_string(),
            "account-b".to_string(),
            " ".to_string(),
        ],
        ..KeroseneConfig::default()
    };

    normalize_loaded_config(&mut config);

    assert_eq!(
        config.pending_keychain_profile_deletions.as_slice(),
        ["account-b"]
    );
    assert!(config.secret_cleanup_state_dirty);
}

#[test]
fn pending_keychain_delete_preserves_active_logical_account_after_prior_removal() {
    let mut config = KeroseneConfig {
        active_account_index: 2,
        accounts: vec![
            profile("account-a"),
            profile("account-b"),
            profile("account-c"),
        ],
        pending_keychain_profile_deletions: vec!["account-a".to_string()],
        ..KeroseneConfig::default()
    };

    normalize_loaded_config(&mut config);

    assert!(config.secret_cleanup_state_dirty);
    assert_eq!(
        config
            .accounts
            .iter()
            .map(|profile| profile.secret_id.as_str())
            .collect::<Vec<_>>(),
        ["account-b", "account-c"]
    );
    assert_eq!(config.active_account_index, 1);
    assert_eq!(
        config.accounts[config.active_account_index].secret_id,
        "account-c"
    );
}

#[test]
fn pending_keychain_delete_prunes_account_scoped_state() {
    let mut config = KeroseneConfig {
        accounts: vec![profile("account-a"), profile("account-b")],
        pending_keychain_profile_deletions: vec![
            "account-b".to_string(),
            "account-b".to_string(),
            " ".to_string(),
        ],
        hidden_positions_by_account: std::collections::HashMap::from([
            ("account-a".to_string(), vec!["ETH".to_string()]),
            ("account-b".to_string(), vec!["BTC".to_string()]),
        ]),
        journal_entries_by_account: std::collections::HashMap::from([
            ("account-a".to_string(), std::collections::HashMap::new()),
            ("account-b".to_string(), std::collections::HashMap::new()),
        ]),
        ..KeroseneConfig::default()
    };

    normalize_loaded_config(&mut config);

    assert!(config.secret_cleanup_state_dirty);
    assert_eq!(config.accounts.len(), 1);
    assert_eq!(config.accounts[0].secret_id, "account-a");
    assert!(config.hidden_positions_by_account.contains_key("account-a"));
    assert!(!config.hidden_positions_by_account.contains_key("account-b"));
    assert!(config.journal_entries_by_account.contains_key("account-a"));
    assert!(!config.journal_entries_by_account.contains_key("account-b"));
    assert_eq!(
        config.pending_keychain_profile_deletions.as_slice(),
        ["account-b"]
    );
}

#[test]
fn prunes_unsupported_panes_from_loaded_layouts() {
    let mut config = KeroseneConfig {
        pane_layout: Some(crate::config::PaneLayoutConfig::Split {
            axis: crate::config::AxisConfig::Vertical,
            ratio: 0.5,
            a: Box::new(crate::config::PaneLayoutConfig::Leaf(
                crate::config::PaneKindConfig::Chart { chart_id: 7 },
            )),
            b: Box::new(crate::config::PaneLayoutConfig::Leaf(
                crate::config::PaneKindConfig::Unsupported,
            )),
        }),
        saved_layouts: vec![
            serde_json::from_value(serde_json::json!({
                "name": "legacy-assistant-only",
                "pane_layout": { "Leaf": "Assistant" }
            }))
            .expect("legacy saved layout should deserialize"),
        ],
        ..KeroseneConfig::default()
    };

    normalize_loaded_config(&mut config);

    assert_eq!(
        config.pane_layout,
        Some(crate::config::PaneLayoutConfig::Leaf(
            crate::config::PaneKindConfig::Chart { chart_id: 7 }
        ))
    );
    assert_eq!(config.saved_layouts[0].pane_layout, None);
}

#[test]
fn normalizes_persisted_split_ratios() {
    let mut config = KeroseneConfig {
        layout_ratios: vec![f32::NAN, -0.25, 0.25, 1.25],
        pane_layout: Some(crate::config::PaneLayoutConfig::Split {
            axis: crate::config::AxisConfig::Vertical,
            ratio: f32::NAN,
            a: Box::new(crate::config::PaneLayoutConfig::Leaf(
                crate::config::PaneKindConfig::Chart { chart_id: 7 },
            )),
            b: Box::new(crate::config::PaneLayoutConfig::Split {
                axis: crate::config::AxisConfig::Horizontal,
                ratio: 2.0,
                a: Box::new(crate::config::PaneLayoutConfig::Leaf(
                    crate::config::PaneKindConfig::OrderBook { id: 1 },
                )),
                b: Box::new(crate::config::PaneLayoutConfig::Leaf(
                    crate::config::PaneKindConfig::Watchlist,
                )),
            }),
        }),
        saved_layouts: vec![
            serde_json::from_value(serde_json::json!({
                "name": "bad-ratios",
                "layout_ratios": [-1.0, 0.4, 2.0],
                "pane_layout": {
                    "Split": {
                        "axis": "Vertical",
                        "ratio": 2.0,
                        "a": { "Leaf": { "Chart": { "chart_id": 7 } } },
                        "b": { "Leaf": "Watchlist" }
                    }
                }
            }))
            .expect("saved layout should deserialize"),
        ],
        ..KeroseneConfig::default()
    };

    normalize_loaded_config(&mut config);

    assert_eq!(config.layout_ratios, vec![0.5, 0.0, 0.25, 1.0]);
    assert_eq!(
        split_ratios(config.pane_layout.as_ref().expect("pane layout")),
        vec![0.5, 1.0]
    );
    assert_eq!(config.saved_layouts[0].layout_ratios, vec![0.0, 0.4, 1.0]);
    assert_eq!(
        split_ratios(
            config.saved_layouts[0]
                .pane_layout
                .as_ref()
                .expect("saved pane layout")
        ),
        vec![1.0]
    );
}

fn split_ratios(layout: &crate::config::PaneLayoutConfig) -> Vec<f32> {
    fn walk(layout: &crate::config::PaneLayoutConfig, ratios: &mut Vec<f32>) {
        match layout {
            crate::config::PaneLayoutConfig::Leaf(_) => {}
            crate::config::PaneLayoutConfig::Split { ratio, a, b, .. } => {
                ratios.push(*ratio);
                walk(a, ratios);
                walk(b, ratios);
            }
        }
    }

    let mut ratios = Vec::new();
    walk(layout, &mut ratios);
    ratios
}

#[test]
fn repairs_duplicate_non_chart_widget_ids() {
    let mut config = KeroseneConfig {
        order_books: vec![order_book_config(0), order_book_config(0)],
        live_watchlists: vec![live_watchlist_config(0), live_watchlist_config(0)],
        positioning_infos: vec![positioning_info_config(0), positioning_info_config(0)],
        session_data: vec![session_data_config(0), session_data_config(0)],
        pane_layout: Some(crate::config::PaneLayoutConfig::Split {
            axis: crate::config::AxisConfig::Vertical,
            ratio: 0.5,
            a: Box::new(crate::config::PaneLayoutConfig::Leaf(
                crate::config::PaneKindConfig::OrderBook { id: 0 },
            )),
            b: Box::new(crate::config::PaneLayoutConfig::Leaf(
                crate::config::PaneKindConfig::OrderBook { id: 0 },
            )),
        }),
        saved_layouts: vec![
            serde_json::from_value(serde_json::json!({
                "name": "duplicate-widgets",
                "order_books": [{ "id": 0 }, { "id": 0 }],
                "live_watchlists": [{ "id": 0 }, { "id": 0 }],
                "positioning_infos": [{ "id": 0 }, { "id": 0 }],
                "session_data": [{ "id": 0 }, { "id": 0 }],
                "pane_layout": {
                    "Split": {
                        "axis": "Vertical",
                        "ratio": 0.5,
                        "a": { "Leaf": { "SessionData": { "id": 0 } } },
                        "b": { "Leaf": { "SessionData": { "id": 0 } } }
                    }
                }
            }))
            .expect("saved layout should deserialize"),
        ],
        ..KeroseneConfig::default()
    };

    normalize_loaded_config(&mut config);

    assert_eq!(
        config
            .order_books
            .iter()
            .map(|cfg| cfg.id)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(
        config
            .live_watchlists
            .iter()
            .map(|cfg| cfg.id)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(
        config
            .positioning_infos
            .iter()
            .map(|cfg| cfg.id)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(
        config
            .session_data
            .iter()
            .map(|cfg| cfg.id)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(
        order_book_leaf_ids(config.pane_layout.as_ref().expect("pane layout")),
        vec![0, 2]
    );

    let saved = &config.saved_layouts[0];
    assert_eq!(
        saved
            .order_books
            .iter()
            .map(|cfg| cfg.id)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(
        saved
            .live_watchlists
            .iter()
            .map(|cfg| cfg.id)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(
        saved
            .positioning_infos
            .iter()
            .map(|cfg| cfg.id)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(
        saved
            .session_data
            .iter()
            .map(|cfg| cfg.id)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(
        session_data_leaf_ids(saved.pane_layout.as_ref().expect("saved pane layout")),
        vec![0, 2]
    );
}

fn order_book_config(id: u64) -> crate::config::OrderBookConfig {
    serde_json::from_value(serde_json::json!({ "id": id })).expect("order book config")
}

fn live_watchlist_config(id: u64) -> crate::config::LiveWatchlistConfig {
    serde_json::from_value(serde_json::json!({ "id": id })).expect("live watchlist config")
}

fn positioning_info_config(id: u64) -> crate::config::PositioningInfoConfig {
    serde_json::from_value(serde_json::json!({ "id": id })).expect("positioning config")
}

fn session_data_config(id: u64) -> crate::config::SessionDataConfig {
    serde_json::from_value(serde_json::json!({ "id": id })).expect("session data config")
}

fn order_book_leaf_ids(layout: &crate::config::PaneLayoutConfig) -> Vec<u64> {
    non_chart_leaf_ids(layout, |kind| match kind {
        crate::config::PaneKindConfig::OrderBook { id } => Some(*id),
        _ => None,
    })
}

fn session_data_leaf_ids(layout: &crate::config::PaneLayoutConfig) -> Vec<u64> {
    non_chart_leaf_ids(layout, |kind| match kind {
        crate::config::PaneKindConfig::SessionData { id } => Some(*id),
        _ => None,
    })
}

fn non_chart_leaf_ids(
    layout: &crate::config::PaneLayoutConfig,
    id_for: fn(&crate::config::PaneKindConfig) -> Option<u64>,
) -> Vec<u64> {
    fn walk(
        layout: &crate::config::PaneLayoutConfig,
        id_for: fn(&crate::config::PaneKindConfig) -> Option<u64>,
        ids: &mut Vec<u64>,
    ) {
        match layout {
            crate::config::PaneLayoutConfig::Leaf(kind) => {
                if let Some(id) = id_for(kind) {
                    ids.push(id);
                }
            }
            crate::config::PaneLayoutConfig::Split { a, b, .. } => {
                walk(a, id_for, ids);
                walk(b, id_for, ids);
            }
        }
    }

    let mut ids = Vec::new();
    walk(layout, id_for, &mut ids);
    ids
}

#[test]
fn preserves_unknown_future_panes_in_loaded_layouts() {
    let raw_future_pane = serde_json::json!({
        "FuturePane": {
            "id": 9,
            "label": "newer-version"
        }
    });
    let future_leaf = crate::config::PaneLayoutConfig::Leaf(
        crate::config::PaneKindConfig::Unknown(raw_future_pane.clone()),
    );
    let chart_leaf =
        crate::config::PaneLayoutConfig::Leaf(crate::config::PaneKindConfig::Chart { chart_id: 7 });
    let future_split = crate::config::PaneLayoutConfig::Split {
        axis: crate::config::AxisConfig::Vertical,
        ratio: 0.5,
        a: Box::new(chart_leaf.clone()),
        b: Box::new(future_leaf),
    };
    let saved_layout = serde_json::from_value(serde_json::json!({
        "name": "future-pane",
        "pane_layout": {
            "Split": {
                "axis": "Vertical",
                "ratio": 0.5,
                "a": { "Leaf": { "Chart": { "chart_id": 7 } } },
                "b": { "Leaf": raw_future_pane }
            }
        }
    }))
    .expect("saved layout with future pane should deserialize");
    let mut config = KeroseneConfig {
        pane_layout: Some(future_split.clone()),
        saved_layouts: vec![saved_layout],
        ..KeroseneConfig::default()
    };

    normalize_loaded_config(&mut config);

    assert_eq!(config.pane_layout, Some(future_split.clone()));
    assert_eq!(config.saved_layouts[0].pane_layout, Some(future_split));
}

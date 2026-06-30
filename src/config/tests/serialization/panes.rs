use super::super::config_warning_guard;
use super::{json_string, value_from_json};
use crate::config::{
    AxisConfig, BottomTabConfig, KeroseneConfig, OrderBookConfig, OrderBookDisplayModeConfig,
    OrderBookSymbolModeConfig, PaneKindConfig, PaneLayoutConfig, take_config_warnings,
};
use crate::session_data_state::SessionDataLookback;

#[test]
fn legacy_assistant_pane_deserializes_as_unsupported() {
    let layout: PaneLayoutConfig = value_from_json(
        serde_json::json!({"Leaf": "Assistant"}),
        "legacy assistant pane should deserialize",
    );

    assert_eq!(layout, PaneLayoutConfig::Leaf(PaneKindConfig::Unsupported));
}

#[test]
fn legacy_x_feed_pane_deserializes_as_unsupported() {
    let layout: PaneLayoutConfig = value_from_json(
        serde_json::json!({"Leaf": "XFeed"}),
        "legacy x feed pane should deserialize",
    );

    assert_eq!(layout, PaneLayoutConfig::Leaf(PaneKindConfig::Unsupported));
}

#[test]
fn x_feed_pane_round_trips_with_instance_id() {
    let layout = PaneLayoutConfig::Leaf(PaneKindConfig::XFeed { id: 42 });

    let json = serde_json::to_value(&layout).expect("x feed pane should serialize");
    let decoded: PaneLayoutConfig = value_from_json(json.clone(), "x feed pane should deserialize");

    assert_eq!(
        json,
        serde_json::json!({
            "Leaf": {
                "XFeed": {
                    "id": 42
                }
            }
        })
    );
    assert_eq!(decoded, layout);
}

#[test]
fn unknown_future_pane_round_trips_raw_json() {
    let raw_pane = serde_json::json!({
        "FuturePane": {
            "id": 42,
            "label": "alpha",
            "metadata": [true, null]
        }
    });
    let raw_layout = serde_json::json!({ "Leaf": raw_pane.clone() });
    let layout: PaneLayoutConfig = value_from_json(
        raw_layout.clone(),
        "future pane should deserialize as raw unknown data",
    );

    assert_eq!(
        layout,
        PaneLayoutConfig::Leaf(PaneKindConfig::Unknown(raw_pane))
    );
    assert_eq!(
        serde_json::to_value(&layout).expect("future pane should serialize"),
        raw_layout
    );
}

#[test]
fn unknown_future_split_axis_defaults_to_horizontal() {
    let _warning_guard = config_warning_guard();
    let layout: PaneLayoutConfig = value_from_json(
        serde_json::json!({
            "Split": {
                "axis": "Diagonal",
                "ratio": 0.5,
                "a": { "Leaf": "Watchlist" },
                "b": { "Leaf": "OrderEntry" }
            }
        }),
        "future split axis should deserialize",
    );

    let PaneLayoutConfig::Split { axis, .. } = layout else {
        panic!("layout should remain a split");
    };

    assert_eq!(axis, AxisConfig::Horizontal);
    assert!(
        take_config_warnings()
            .iter()
            .any(|warning| warning.contains("Unknown pane split axis \"Diagonal\""))
    );
}

#[test]
fn unknown_future_bottom_tab_defaults_to_positions() {
    let _warning_guard = config_warning_guard();
    let kind: PaneKindConfig = value_from_json(
        serde_json::json!({ "BottomTabs": { "active_tab": "FutureTab" } }),
        "future bottom tab should deserialize",
    );

    assert_eq!(
        kind,
        PaneKindConfig::BottomTabs {
            active_tab: BottomTabConfig::Positions
        }
    );
    assert!(
        take_config_warnings()
            .iter()
            .any(|warning| warning.contains("Unknown bottom tab \"FutureTab\""))
    );
}

#[test]
fn unknown_future_order_book_modes_default_with_warnings() {
    let _warning_guard = config_warning_guard();
    let order_book: OrderBookConfig = value_from_json(
        serde_json::json!({
            "id": 7,
            "mode": "FutureMode",
            "tick_size": 1.0,
            "display_mode": "FutureDisplayMode"
        }),
        "future order book modes should deserialize",
    );

    assert_eq!(order_book.mode, OrderBookSymbolModeConfig::Active);
    assert_eq!(
        order_book.display_mode,
        OrderBookDisplayModeConfig::DepthList
    );

    let warnings = take_config_warnings();
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("Unknown order book symbol mode \"FutureMode\""))
    );
    assert!(
        warnings.iter().any(
            |warning| warning.contains("Unknown order book display mode \"FutureDisplayMode\"")
        )
    );
}

#[test]
fn known_pane_variants_keep_existing_wire_shape() {
    let cases = vec![
        (
            PaneKindConfig::AccountSummary,
            serde_json::json!("AccountSummary"),
        ),
        (
            PaneKindConfig::Chart { chart_id: 7 },
            serde_json::json!({ "Chart": { "chart_id": 7 } }),
        ),
        (
            PaneKindConfig::OrderBook { id: 8 },
            serde_json::json!({ "OrderBook": { "id": 8 } }),
        ),
        (PaneKindConfig::Watchlist, serde_json::json!("Watchlist")),
        (
            PaneKindConfig::LiveWatchlist { id: 9 },
            serde_json::json!({ "LiveWatchlist": { "id": 9 } }),
        ),
        (
            PaneKindConfig::PositioningInfo { id: 10 },
            serde_json::json!({ "PositioningInfo": { "id": 10 } }),
        ),
        (
            PaneKindConfig::SessionData { id: 11 },
            serde_json::json!({ "SessionData": { "id": 11 } }),
        ),
        (
            PaneKindConfig::XFeed { id: 13 },
            serde_json::json!({ "XFeed": { "id": 13 } }),
        ),
        (PaneKindConfig::Portfolio, serde_json::json!("Portfolio")),
        (PaneKindConfig::Income, serde_json::json!("Income")),
        (
            PaneKindConfig::BottomTabs {
                active_tab: BottomTabConfig::FundingHistory,
            },
            serde_json::json!({ "BottomTabs": { "active_tab": "FundingHistory" } }),
        ),
        (PaneKindConfig::OrderEntry, serde_json::json!("OrderEntry")),
        (
            PaneKindConfig::AdvancedOrders,
            serde_json::json!("AdvancedOrders"),
        ),
        (
            PaneKindConfig::SpaghettiChart { spaghetti_id: 12 },
            serde_json::json!({ "SpaghettiChart": { "spaghetti_id": 12 } }),
        ),
        (PaneKindConfig::Settings, serde_json::json!("Settings")),
        (PaneKindConfig::Calendar, serde_json::json!("Calendar")),
        (
            PaneKindConfig::Liquidations,
            serde_json::json!("Liquidations"),
        ),
        (
            PaneKindConfig::LiquidationsDistribution,
            serde_json::json!("LiquidationsDistribution"),
        ),
        (
            PaneKindConfig::TrackedTrades,
            serde_json::json!("TrackedTrades"),
        ),
        (
            PaneKindConfig::TelegramFeed,
            serde_json::json!("TelegramFeed"),
        ),
        (PaneKindConfig::Outcomes, serde_json::json!("Outcomes")),
        (PaneKindConfig::HypeEtfs, serde_json::json!("HypeEtfs")),
        (
            PaneKindConfig::HypeUnstakingQueue,
            serde_json::json!("HypeUnstakingQueue"),
        ),
        (
            PaneKindConfig::Unsupported,
            serde_json::json!("Unsupported"),
        ),
    ];

    for (kind, expected_json) in cases {
        assert_eq!(
            serde_json::to_value(&kind).expect("pane kind should serialize"),
            expected_json,
            "serialized wire shape changed for {kind:?}"
        );
        assert_eq!(
            serde_json::from_value::<PaneKindConfig>(expected_json.clone())
                .expect("pane kind should deserialize"),
            kind,
            "deserialized wire shape changed for {expected_json}"
        );
    }
}

#[test]
fn known_pane_future_fields_are_intentionally_dropped() {
    let raw_pane = serde_json::json!({
        "Chart": {
            "chart_id": 7,
            "future_field": {
                "kept_by_newer_versions": true
            }
        }
    });
    let decoded: PaneKindConfig = value_from_json(
        raw_pane,
        "known pane with future fields should deserialize using the known schema",
    );

    assert_eq!(decoded, PaneKindConfig::Chart { chart_id: 7 });
    assert_eq!(
        serde_json::to_value(decoded).expect("known pane should serialize"),
        serde_json::json!({ "Chart": { "chart_id": 7 } })
    );
}

#[test]
fn telegram_feed_pane_round_trips() {
    let layout = PaneLayoutConfig::Leaf(PaneKindConfig::TelegramFeed);

    let json = json_string(&layout, "telegram feed pane should serialize");
    let decoded: PaneLayoutConfig =
        serde_json::from_str(&json).expect("telegram feed pane should deserialize");

    assert_eq!(decoded, layout);
}

#[test]
fn session_data_pane_and_config_round_trip() {
    let layout = PaneLayoutConfig::Leaf(PaneKindConfig::SessionData { id: 42 });

    let json = json_string(&layout, "session data pane should serialize");
    let decoded: PaneLayoutConfig =
        serde_json::from_str(&json).expect("session data pane should deserialize");

    assert_eq!(decoded, layout);

    let mut config = KeroseneConfig::default();
    config.session_data.push(crate::config::SessionDataConfig {
        id: 42,
        symbol: "@107".to_string(),
        lookback: SessionDataLookback::EightWeeks,
    });
    let json = json_string(&config, "session data config should serialize");
    let decoded: KeroseneConfig =
        serde_json::from_str(&json).expect("session data config should deserialize");

    assert_eq!(decoded.session_data.len(), 1);
    assert_eq!(decoded.session_data[0].id, 42);
    assert_eq!(decoded.session_data[0].symbol, "@107");
    assert_eq!(
        decoded.session_data[0].lookback,
        SessionDataLookback::EightWeeks
    );
}

#[test]
fn serialized_config_omits_removed_assistant_settings() {
    let json = json_string(
        &KeroseneConfig::default(),
        "default config should serialize",
    );

    assert!(!json.contains("assistant_api_key"));
    assert!(!json.contains("assistant_model"));
    assert!(!json.contains("assistant_use_account_context"));
    assert!(!json.contains("assistant_allow_code_execution"));
}

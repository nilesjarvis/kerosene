use super::super::config_warning_guard;
use super::{default_config_value, json_string, object_mut, value_from_json, value_from_str};
use crate::config::{
    KeroseneConfig, LiveWatchlistColumn, LiveWatchlistConfig, LiveWatchlistEmaConfig,
    LiveWatchlistSortColumn, SavedLayout, SortDirection, SpaghettiChartConfig,
    WatchlistPresetConfig, default_live_watchlist_columns, take_config_warnings,
};

#[test]
fn live_watchlists_round_trip() {
    let config = KeroseneConfig {
        live_watchlists: vec![LiveWatchlistConfig {
            id: 42,
            preset_id: Some(9),
            symbols: vec!["BTC".to_string(), "xyz:NVDA".to_string()],
            sort_column: LiveWatchlistSortColumn::EmaDistance,
            sort_direction: SortDirection::Descending,
            ema: crate::config::LiveWatchlistEmaConfig {
                period: 200,
                timeframe: "4h".to_string(),
            },
            visible_columns: vec![
                LiveWatchlistColumn::Price,
                LiveWatchlistColumn::EmaDistance,
                LiveWatchlistColumn::Funding,
            ],
        }],
        watchlist_presets: vec![WatchlistPresetConfig {
            id: 9,
            name: "US Tech".to_string(),
            symbols: vec!["BTC".to_string(), "xyz:NVDA".to_string()],
        }],
        spaghetti_charts: vec![SpaghettiChartConfig {
            watchlist_preset_id: Some(9),
            symbols: vec!["BTC".to_string(), "xyz:NVDA".to_string()],
            ..SpaghettiChartConfig::empty(7)
        }],
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");

    assert_eq!(decoded.live_watchlists, config.live_watchlists);
    assert_eq!(decoded.watchlist_presets, config.watchlist_presets);
    assert_eq!(decoded.spaghetti_charts, config.spaghetti_charts);
}

#[test]
fn live_watchlists_legacy_defaults_are_backwards_compatible() {
    let mut missing_top_level = default_config_value();
    object_mut(&mut missing_top_level, "config should serialize to object")
        .remove("live_watchlists");
    let decoded_missing: KeroseneConfig =
        value_from_json(missing_top_level, "legacy config should deserialize");
    assert!(decoded_missing.live_watchlists.is_empty());
    assert!(decoded_missing.watchlist_presets.is_empty());

    let mut legacy_config = default_config_value();
    object_mut(&mut legacy_config, "config should serialize to object").insert(
        "live_watchlists".to_string(),
        serde_json::json!([{ "id": 7, "symbols": ["BTC"] }]),
    );
    let decoded_config: KeroseneConfig =
        value_from_json(legacy_config, "legacy config should deserialize");
    let decoded_config_watchlist = decoded_config
        .live_watchlists
        .first()
        .expect("legacy config live watchlist");
    assert_eq!(
        decoded_config_watchlist.sort_column,
        LiveWatchlistSortColumn::Symbol
    );
    assert_eq!(
        decoded_config_watchlist.sort_direction,
        SortDirection::Ascending
    );
    assert_eq!(
        decoded_config_watchlist.visible_columns,
        default_live_watchlist_columns()
    );

    let legacy_watchlist = serde_json::json!({
        "id": 7,
        "symbols": ["BTC"],
        "sort_column": "Change1h",
        "sort_direction": "Descending"
    });
    let decoded_watchlist: LiveWatchlistConfig =
        value_from_json(legacy_watchlist, "legacy live watchlist should deserialize");

    assert_eq!(decoded_watchlist.id, 7);
    assert_eq!(decoded_watchlist.preset_id, None);
    assert_eq!(decoded_watchlist.symbols, vec!["BTC".to_string()]);
    assert_eq!(
        decoded_watchlist.sort_column,
        LiveWatchlistSortColumn::Change1h
    );
    assert_eq!(decoded_watchlist.sort_direction, SortDirection::Descending);
    assert_eq!(
        decoded_watchlist.visible_columns,
        default_live_watchlist_columns()
    );

    let saved_layout: SavedLayout = value_from_json(
        serde_json::json!({
            "name": "Legacy",
            "live_watchlists": [{ "id": 9, "symbols": ["ETH"] }]
        }),
        "legacy saved layout should deserialize",
    );
    let saved_watchlist = saved_layout
        .live_watchlists
        .first()
        .expect("legacy saved layout live watchlist");
    assert_eq!(saved_watchlist.id, 9);
    assert_eq!(saved_watchlist.preset_id, None);
    assert_eq!(saved_watchlist.symbols, vec!["ETH".to_string()]);
    assert_eq!(saved_watchlist.sort_column, LiveWatchlistSortColumn::Symbol);
    assert_eq!(saved_watchlist.sort_direction, SortDirection::Ascending);
    assert_eq!(
        saved_watchlist.visible_columns,
        default_live_watchlist_columns()
    );
}

#[test]
fn live_watchlists_default_or_drop_unknown_persisted_enum_values() {
    let _warning_guard = config_warning_guard();
    let mut config = default_config_value();
    object_mut(&mut config, "config should serialize to object").insert(
        "live_watchlists".to_string(),
        serde_json::json!([
            {
                "id": 11,
                "symbols": ["BTC"],
                "sort_column": "FutureSort",
                "sort_direction": "FutureDirection",
                "visible_columns": ["Price", "FutureColumn", "Funding"]
            }
        ]),
    );

    let decoded: KeroseneConfig =
        value_from_json(config, "future live watchlist config should deserialize");
    let watchlist = decoded
        .live_watchlists
        .first()
        .expect("decoded live watchlist");

    assert_eq!(watchlist.sort_column, LiveWatchlistSortColumn::Symbol);
    assert_eq!(watchlist.sort_direction, SortDirection::Ascending);
    assert_eq!(
        watchlist.visible_columns,
        vec![LiveWatchlistColumn::Price, LiveWatchlistColumn::Funding]
    );

    let warnings = take_config_warnings();
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("Unknown live watchlist sort column \"FutureSort\""))
    );
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("Unknown sort direction \"FutureDirection\""))
    );
    assert!(
        warnings
            .iter()
            .any(|warning| warning
                .contains("Unknown live watchlist visible column \"FutureColumn\""))
    );
}

#[test]
fn live_watchlist_ema_defaults_preserve_legacy_columns_and_normalize_settings() {
    let legacy: LiveWatchlistConfig =
        value_from_json(serde_json::json!({"id": 1}), "legacy config");
    assert_eq!(legacy.ema, LiveWatchlistEmaConfig::default());
    assert_eq!(legacy.visible_columns.len(), 6);
    assert!(
        !legacy
            .visible_columns
            .contains(&LiveWatchlistColumn::EmaDistance)
    );
    for (raw, period, timeframe) in [
        (serde_json::json!({}), 20, "1h"),
        (
            serde_json::json!({"period": 0, "timeframe": "tick"}),
            1,
            "1h",
        ),
        (
            serde_json::json!({"period": 999999, "timeframe": "1s"}),
            1000,
            "1h",
        ),
        (
            serde_json::json!({"period": 50, "timeframe": "1M"}),
            50,
            "1M",
        ),
    ] {
        let parsed: LiveWatchlistEmaConfig = value_from_json(raw, "EMA settings");
        assert_eq!(parsed.period, period);
        assert_eq!(parsed.timeframe, timeframe);
    }
}

#[test]
fn live_watchlist_ema_saved_layout_round_trip() {
    let layout: SavedLayout = value_from_json(
        serde_json::json!({
            "name": "EMA", "live_watchlists": [{"id": 1,
            "ema": {"period": 50, "timeframe": "1d"},
            "visible_columns": ["Price", "EmaDistance"], "sort_column": "EmaDistance"}]
        }),
        "layout",
    );
    let decoded: SavedLayout = value_from_str(&json_string(&layout, "serialize"), "deserialize");
    assert_eq!(decoded.live_watchlists, layout.live_watchlists);
}

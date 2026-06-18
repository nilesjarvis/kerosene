use super::super::config_warning_guard;
use super::{default_config_value, json_string, object_mut, value_from_json, value_from_str};
use crate::config::{
    KeroseneConfig, LiveWatchlistColumn, LiveWatchlistConfig, LiveWatchlistSortColumn, SavedLayout,
    SortDirection, default_live_watchlist_columns, take_config_warnings,
};

#[test]
fn live_watchlists_round_trip() {
    let config = KeroseneConfig {
        live_watchlists: vec![LiveWatchlistConfig {
            id: 42,
            symbols: vec!["BTC".to_string(), "xyz:NVDA".to_string()],
            sort_column: LiveWatchlistSortColumn::Change24h,
            sort_direction: SortDirection::Descending,
            visible_columns: vec![LiveWatchlistColumn::Price, LiveWatchlistColumn::Funding],
        }],
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");

    assert_eq!(decoded.live_watchlists, config.live_watchlists);
}

#[test]
fn live_watchlists_legacy_defaults_are_backwards_compatible() {
    let mut missing_top_level = default_config_value();
    object_mut(&mut missing_top_level, "config should serialize to object")
        .remove("live_watchlists");
    let decoded_missing: KeroseneConfig =
        value_from_json(missing_top_level, "legacy config should deserialize");
    assert!(decoded_missing.live_watchlists.is_empty());

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

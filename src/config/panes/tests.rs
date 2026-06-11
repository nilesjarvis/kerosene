use super::{
    DetachedChartWindowConfig, OrderBookConfig, OrderBookDisplayModeConfig, PositioningInfoConfig,
};
use crate::config::SortDirection;
use crate::positioning_state::PositioningInfoPage;

#[test]
fn detached_chart_window_config_uses_multi_monitor_defaults() {
    let config: DetachedChartWindowConfig =
        serde_json::from_str(r#"{"chart_id":7}"#).expect("config");

    assert_eq!(config.chart_id, 7);
    assert_eq!(config.width, 1100.0);
    assert_eq!(config.height, 720.0);
    assert_eq!(config.x, None);
    assert_eq!(config.y, None);
}

#[test]
fn order_book_config_defaults_to_depth_list_display_mode() {
    let config: OrderBookConfig =
        serde_json::from_str(r#"{"id":7,"tick_size":1.0}"#).expect("config");

    assert_eq!(config.display_mode, OrderBookDisplayModeConfig::DepthList);
    // Centered-on-mid is the default for configs that predate the field.
    assert!(config.center_on_mid);
    assert!(!config.reverse_side);
}

#[test]
fn order_book_config_round_trips_dom_ladder_display_mode() {
    let config: OrderBookConfig = serde_json::from_str(
        r#"{
            "id": 7,
            "tick_size": 1.0,
            "display_mode": "DomLadder",
            "center_on_mid": true,
            "reverse_side": true
        }"#,
    )
    .expect("config");

    let rendered = serde_json::to_string(&config).expect("json");
    assert!(rendered.contains(r#""display_mode":"DomLadder""#));
    assert!(rendered.contains(r#""center_on_mid":true"#));
    assert!(rendered.contains(r#""reverse_side":true"#));
}

#[test]
fn order_book_config_round_trips_depth_chart_display_mode() {
    let config: OrderBookConfig =
        serde_json::from_str(r#"{"id":7,"tick_size":1.0,"display_mode":"DepthChart"}"#)
            .expect("config");

    assert_eq!(config.display_mode, OrderBookDisplayModeConfig::DepthChart);
    let rendered = serde_json::to_string(&config).expect("json");
    assert!(rendered.contains(r#""display_mode":"DepthChart""#));
}

#[test]
fn positioning_info_config_defaults_to_descending_sort() {
    let config: PositioningInfoConfig =
        serde_json::from_str(r#"{"id":7,"symbol":"HYPE"}"#).expect("config");

    assert_eq!(config.sort_direction, SortDirection::Descending);
    assert_eq!(config.change_sort_direction, SortDirection::Descending);
    assert_eq!(config.page, PositioningInfoPage::Positions);
}

#[test]
fn positioning_info_config_round_trips_active_page() {
    let config: PositioningInfoConfig =
        serde_json::from_str(r#"{"id":7,"symbol":"HYPE","page":"Change"}"#).expect("config");

    assert_eq!(config.page, PositioningInfoPage::Change);
    let rendered = serde_json::to_string(&config).expect("json");
    assert!(rendered.contains(r#""page":"Change""#));
}

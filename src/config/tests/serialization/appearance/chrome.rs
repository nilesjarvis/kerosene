use super::{
    ChartBackfillSource, ChartCrosshairStyle, ChartHollowCandleMode, ChartHudReadoutConfig,
    KeroseneConfig, WidgetPaddingConfig, WidgetPaddingOverrideConfig, WidgetPaddingTargetConfig,
    default_alfred_popup_scale, default_chart_chromatic_aberration_strength,
    default_chart_crosshair_scale, default_chart_dotted_background_opacity,
    default_chart_edge_blur_strength, default_chart_fisheye_strength, default_config_value,
    default_pane_border_thickness, default_pane_corner_radius, default_ui_scale,
    default_widget_padding, json_string, object_mut, value_from_json, value_from_str,
};
use crate::config::ReadDataProvider;

#[test]
fn widget_chrome_round_trips_and_legacy_defaults_current_values() {
    let config = KeroseneConfig {
        ui_scale: 0.85,
        chart_dotted_background: true,
        chart_dotted_background_opacity: 0.27,
        chart_hollow_candle_mode: ChartHollowCandleMode::Both,
        chart_fisheye_enabled: true,
        chart_fisheye_strength: 0.72,
        chart_chromatic_aberration_enabled: true,
        chart_chromatic_aberration_strength: 0.66,
        chart_edge_blur_enabled: true,
        chart_edge_blur_strength: 0.57,
        chart_crosshair_style: ChartCrosshairStyle::RacingHud,
        chart_crosshair_guides_enabled: false,
        chart_crosshair_scale: 1.55,
        chart_hud_ui_sounds: false,
        chart_hud_readout: ChartHudReadoutConfig {
            price: false,
            clock: false,
            ..ChartHudReadoutConfig::default()
        },
        alfred_popup_scale: 1.35,
        read_data_provider: ReadDataProvider::Hydromancer,
        pane_border_thickness: 8.0,
        pane_corner_radius: 12.0,
        outer_widget_border_enabled: true,
        widget_padding: WidgetPaddingConfig {
            default_px: 6.0,
            overrides: vec![WidgetPaddingOverrideConfig {
                target: WidgetPaddingTargetConfig::Watchlist,
                padding_px: 14.0,
            }],
        },
        custom_window_chrome_enabled: false,
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert_eq!(decoded.ui_scale, 0.85);
    assert!(decoded.chart_dotted_background);
    assert_eq!(decoded.chart_dotted_background_opacity, 0.27);
    assert_eq!(
        decoded.chart_hollow_candle_mode,
        ChartHollowCandleMode::Both
    );
    assert!(decoded.chart_fisheye_enabled);
    assert_eq!(decoded.chart_fisheye_strength, 0.72);
    assert!(decoded.chart_chromatic_aberration_enabled);
    assert_eq!(decoded.chart_chromatic_aberration_strength, 0.66);
    assert!(decoded.chart_edge_blur_enabled);
    assert_eq!(decoded.chart_edge_blur_strength, 0.57);
    assert_eq!(
        decoded.chart_crosshair_style,
        ChartCrosshairStyle::RacingHud
    );
    assert!(!decoded.chart_crosshair_guides_enabled);
    assert_eq!(decoded.chart_crosshair_scale, 1.55);
    assert!(!decoded.chart_hud_ui_sounds);
    assert!(decoded.chart_hud_readout.symbol);
    assert!(!decoded.chart_hud_readout.price);
    assert!(decoded.chart_hud_readout.coordinates);
    assert!(decoded.chart_hud_readout.hover_time);
    assert!(!decoded.chart_hud_readout.clock);
    assert!(decoded.chart_hud_readout.candle_close);
    assert_eq!(decoded.alfred_popup_scale, 1.35);
    assert_eq!(decoded.read_data_provider, ReadDataProvider::Hydromancer);
    assert_eq!(
        decoded.chart_backfill_source,
        ChartBackfillSource::Hyperliquid
    );
    assert_eq!(decoded.pane_border_thickness, 8.0);
    assert_eq!(decoded.pane_corner_radius, 12.0);
    assert!(decoded.outer_widget_border_enabled);
    assert_eq!(decoded.widget_padding.default_px, 6.0);
    assert_eq!(decoded.widget_padding.overrides.len(), 1);
    assert_eq!(
        decoded.widget_padding.overrides[0].target,
        WidgetPaddingTargetConfig::Watchlist
    );
    assert_eq!(decoded.widget_padding.overrides[0].padding_px, 14.0);
    assert!(!decoded.custom_window_chrome_enabled);

    let mut legacy = default_config_value();
    let object = object_mut(&mut legacy, "config should serialize to object");
    object.remove("ui_scale");
    object.remove("chart_dotted_background");
    object.remove("chart_dotted_background_opacity");
    object.remove("chart_hollow_candle_mode");
    object.remove("chart_fisheye_enabled");
    object.remove("chart_fisheye_strength");
    object.remove("chart_chromatic_aberration_enabled");
    object.remove("chart_chromatic_aberration_strength");
    object.remove("chart_edge_blur_enabled");
    object.remove("chart_edge_blur_strength");
    object.remove("chart_crosshair_style");
    object.remove("chart_crosshair_guides_enabled");
    object.remove("chart_crosshair_scale");
    object.remove("chart_hud_ui_sounds");
    object.remove("chart_hud_readout");
    object.remove("alfred_popup_scale");
    object.remove("read_data_provider");
    object.remove("chart_backfill_source");
    object.remove("pane_border_thickness");
    object.remove("pane_corner_radius");
    object.remove("outer_widget_border_enabled");
    object.remove("widget_padding");
    object.remove("custom_window_chrome_enabled");

    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert_eq!(decoded_legacy.ui_scale, default_ui_scale());
    assert!(!decoded_legacy.chart_dotted_background);
    assert_eq!(
        decoded_legacy.chart_dotted_background_opacity,
        default_chart_dotted_background_opacity()
    );
    assert!(!decoded_legacy.chart_hollow_candles);
    assert_eq!(
        decoded_legacy.chart_hollow_candle_mode,
        ChartHollowCandleMode::Off
    );
    assert!(!decoded_legacy.chart_fisheye_enabled);
    assert_eq!(
        decoded_legacy.chart_fisheye_strength,
        default_chart_fisheye_strength()
    );
    assert!(!decoded_legacy.chart_chromatic_aberration_enabled);
    assert_eq!(
        decoded_legacy.chart_chromatic_aberration_strength,
        default_chart_chromatic_aberration_strength()
    );
    assert!(!decoded_legacy.chart_edge_blur_enabled);
    assert_eq!(
        decoded_legacy.chart_edge_blur_strength,
        default_chart_edge_blur_strength()
    );
    assert_eq!(
        decoded_legacy.chart_crosshair_style,
        ChartCrosshairStyle::default()
    );
    assert!(decoded_legacy.chart_crosshair_guides_enabled);
    assert_eq!(
        decoded_legacy.chart_crosshair_scale,
        default_chart_crosshair_scale()
    );
    assert!(decoded_legacy.chart_hud_ui_sounds);
    assert_eq!(
        decoded_legacy.chart_hud_readout,
        ChartHudReadoutConfig::default()
    );
    assert_eq!(
        decoded_legacy.alfred_popup_scale,
        default_alfred_popup_scale()
    );
    assert_eq!(
        decoded_legacy.read_data_provider,
        ReadDataProvider::Hyperliquid
    );
    assert_eq!(
        decoded_legacy.chart_backfill_source,
        ChartBackfillSource::Hyperliquid
    );
    assert_eq!(
        decoded_legacy.pane_border_thickness,
        default_pane_border_thickness()
    );
    assert_eq!(
        decoded_legacy.pane_corner_radius,
        default_pane_corner_radius()
    );
    assert!(decoded_legacy.outer_widget_border_enabled);
    assert_eq!(
        decoded_legacy.widget_padding.default_px,
        default_widget_padding()
    );
    assert!(decoded_legacy.widget_padding.overrides.is_empty());
    assert!(decoded_legacy.custom_window_chrome_enabled);
}

#[test]
fn widget_padding_unknown_targets_are_dropped_for_config_and_saved_layouts() {
    let mut value = default_config_value();
    let object = object_mut(&mut value, "config should serialize to object");
    object.insert(
        "widget_padding".to_string(),
        serde_json::json!({
            "default_px": 5.0,
            "overrides": [
                {
                    "target": "Watchlist",
                    "padding_px": 12.0
                },
                {
                    "target": "FutureWidget",
                    "padding_px": 16.0
                },
                {
                    "target": {
                        "FuturePane": {
                            "id": 99
                        }
                    },
                    "padding_px": 18.0
                }
            ]
        }),
    );
    object.insert(
        "saved_layouts".to_string(),
        serde_json::json!([
            {
                "name": "future-padding",
                "widget_padding": {
                    "default_px": 4.0,
                    "overrides": [
                        {
                            "target": "OrderEntry",
                            "padding_px": 10.0
                        },
                        {
                            "target": "RemovedPane",
                            "padding_px": 11.0
                        }
                    ]
                }
            }
        ]),
    );

    let decoded: KeroseneConfig = value_from_json(
        value,
        "config with unknown widget padding targets should deserialize",
    );

    assert_eq!(decoded.widget_padding.default_px, 5.0);
    assert_eq!(decoded.widget_padding.overrides.len(), 1);
    assert_eq!(
        decoded.widget_padding.overrides[0].target,
        WidgetPaddingTargetConfig::Watchlist
    );
    assert_eq!(decoded.widget_padding.overrides[0].padding_px, 12.0);

    let layout_padding = &decoded.saved_layouts[0].widget_padding;
    assert_eq!(layout_padding.default_px, 4.0);
    assert_eq!(layout_padding.overrides.len(), 1);
    assert_eq!(
        layout_padding.overrides[0].target,
        WidgetPaddingTargetConfig::OrderEntry
    );
    assert_eq!(layout_padding.overrides[0].padding_px, 10.0);
}

#[test]
fn widget_padding_malformed_known_override_still_errors() {
    for overrides in [
        serde_json::json!([
            {
                "target": "Watchlist",
                "padding_px": {
                    "not": "a number"
                }
            }
        ]),
        serde_json::json!([
            {
                "target": {
                    "Chart": {
                        "chart_id": "bad"
                    }
                },
                "padding_px": 12.0
            }
        ]),
        serde_json::json!([
            {
                "target": {
                    "Watchlist": {}
                },
                "padding_px": 12.0
            }
        ]),
        serde_json::json!([
            {
                "padding_px": 12.0
            }
        ]),
        serde_json::json!([
            {
                "target": null,
                "padding_px": 12.0
            }
        ]),
    ] {
        let mut value = default_config_value();
        let object = object_mut(&mut value, "config should serialize to object");
        object.insert(
            "widget_padding".to_string(),
            serde_json::json!({
                "default_px": 5.0,
                "overrides": overrides
            }),
        );

        assert!(serde_json::from_value::<KeroseneConfig>(value).is_err());
    }

    assert!(
        serde_json::from_str::<KeroseneConfig>(
            r#"{
                "widget_padding": {
                    "default_px": 5.0,
                    "overrides": [
                        {
                            "target": {
                                "FuturePane": {},
                                "Chart": {
                                    "chart_id": "bad"
                                }
                            },
                            "padding_px": 12.0
                        }
                    ]
                }
            }"#
        )
        .is_err()
    );
}

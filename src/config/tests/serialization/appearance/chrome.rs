use super::{
    ChartBackfillSource, ChartCrosshairStyle, ChartHollowCandleMode, KeroseneConfig,
    default_alfred_popup_scale, default_chart_chromatic_aberration_strength,
    default_chart_crosshair_scale, default_chart_dotted_background_opacity,
    default_chart_edge_blur_strength, default_chart_fisheye_strength, default_config_value,
    default_pane_border_thickness, default_pane_corner_radius, default_ui_scale, json_string,
    object_mut, value_from_json, value_from_str,
};

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
        chart_crosshair_style: ChartCrosshairStyle::Rangefinder,
        chart_crosshair_guides_enabled: false,
        chart_crosshair_scale: 1.55,
        alfred_popup_scale: 1.35,
        chart_backfill_source: ChartBackfillSource::Hydromancer,
        pane_border_thickness: 8.0,
        pane_corner_radius: 12.0,
        outer_widget_border_enabled: true,
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
        ChartCrosshairStyle::Rangefinder
    );
    assert!(!decoded.chart_crosshair_guides_enabled);
    assert_eq!(decoded.chart_crosshair_scale, 1.55);
    assert_eq!(decoded.alfred_popup_scale, 1.35);
    assert_eq!(
        decoded.chart_backfill_source,
        ChartBackfillSource::Hydromancer
    );
    assert_eq!(decoded.pane_border_thickness, 8.0);
    assert_eq!(decoded.pane_corner_radius, 12.0);
    assert!(decoded.outer_widget_border_enabled);
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
    object.remove("alfred_popup_scale");
    object.remove("chart_backfill_source");
    object.remove("pane_border_thickness");
    object.remove("pane_corner_radius");
    object.remove("outer_widget_border_enabled");
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
    assert_eq!(
        decoded_legacy.alfred_popup_scale,
        default_alfred_popup_scale()
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
    assert!(decoded_legacy.custom_window_chrome_enabled);
}

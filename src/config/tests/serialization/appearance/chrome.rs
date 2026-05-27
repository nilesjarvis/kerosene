use super::{
    KeroseneConfig, default_alfred_popup_scale, default_chart_dotted_background_opacity,
    default_config_value, default_pane_border_thickness, default_pane_corner_radius,
    default_ui_scale, json_string, object_mut, value_from_json, value_from_str,
};

#[test]
fn widget_chrome_round_trips_and_legacy_defaults_current_values() {
    let config = KeroseneConfig {
        ui_scale: 0.85,
        chart_dotted_background: true,
        chart_dotted_background_opacity: 0.27,
        alfred_popup_scale: 1.35,
        pane_border_thickness: 8.0,
        pane_corner_radius: 12.0,
        outer_widget_border_enabled: true,
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert_eq!(decoded.ui_scale, 0.85);
    assert!(decoded.chart_dotted_background);
    assert_eq!(decoded.chart_dotted_background_opacity, 0.27);
    assert_eq!(decoded.alfred_popup_scale, 1.35);
    assert_eq!(decoded.pane_border_thickness, 8.0);
    assert_eq!(decoded.pane_corner_radius, 12.0);
    assert!(decoded.outer_widget_border_enabled);

    let mut legacy = default_config_value();
    let object = object_mut(&mut legacy, "config should serialize to object");
    object.remove("ui_scale");
    object.remove("chart_dotted_background");
    object.remove("chart_dotted_background_opacity");
    object.remove("alfred_popup_scale");
    object.remove("pane_border_thickness");
    object.remove("pane_corner_radius");
    object.remove("outer_widget_border_enabled");

    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert_eq!(decoded_legacy.ui_scale, default_ui_scale());
    assert!(!decoded_legacy.chart_dotted_background);
    assert_eq!(
        decoded_legacy.chart_dotted_background_opacity,
        default_chart_dotted_background_opacity()
    );
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
    assert!(decoded_legacy.outer_widget_border_enabled);
}

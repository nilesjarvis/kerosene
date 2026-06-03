use super::*;

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
        chart_fisheye_strength: 99.0,
        chart_chromatic_aberration_strength: 99.0,
        chart_edge_blur_strength: 99.0,
        pane_border_thickness: 99.0,
        pane_corner_radius: f32::NAN,
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

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
        config.pane_border_thickness,
        normalize_pane_border_thickness(99.0)
    );
    assert_eq!(
        config.pane_corner_radius,
        crate::config::default_pane_corner_radius()
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
fn refreshes_known_default_ubuntu_theme() {
    let mut config = KeroseneConfig::default();
    let theme = config
        .custom_themes
        .iter_mut()
        .find(|theme| theme.name == "ubuntu")
        .expect("ubuntu theme should be present");
    theme.background = "#2C001E".to_string();

    normalize_loaded_config(&mut config);

    let theme = config
        .custom_themes
        .iter()
        .find(|theme| theme.name == "ubuntu")
        .expect("ubuntu theme should still be present");

    assert_eq!(theme.background, "#56334B");
}

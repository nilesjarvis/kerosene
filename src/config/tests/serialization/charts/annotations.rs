use super::{ChartConfig, KeroseneConfig, MacroIndicatorsConfig, json_string, value_from_str};
use crate::annotations::{AnchorConfig, Annotation, AnnotationConfig, LineStyleConfig};

fn chart_with(annotations: Vec<AnnotationConfig>) -> ChartConfig {
    ChartConfig {
        id: 7,
        symbol: "BTC".to_string(),
        timeframe: "H1".to_string(),
        annotations,
        inverted: false,
        show_trade_markers: false,
        show_earnings_markers: false,
        header_collapsed: false,
        drawing_toolbar_collapsed: false,
        funding_panel_height: 56,
        session_panel_height: 72,
        macro_indicators: MacroIndicatorsConfig::default(),
        open_interest_as_notional: false,
        asset_volume_as_notional: true,
        outcome_volume_as_notional: false,
    }
}

#[test]
fn mixed_annotation_kinds_round_trip_through_config() {
    let annotations = vec![
        AnnotationConfig {
            kind: "level".to_string(),
            color: [0.4, 0.5, 0.6],
            price: Some(100.0),
            ..AnnotationConfig::default()
        },
        AnnotationConfig {
            kind: "rect".to_string(),
            color: [0.4, 0.5, 0.6],
            anchors: vec![AnchorConfig { t: 1, p: 5.0 }, AnchorConfig { t: 2, p: 6.0 }],
            ..AnnotationConfig::default()
        },
        AnnotationConfig {
            kind: "fib_retracement".to_string(),
            color: [0.9, 0.9, 0.2],
            anchors: vec![AnchorConfig { t: 1, p: 5.0 }, AnchorConfig { t: 2, p: 9.0 }],
            ..AnnotationConfig::default()
        },
        AnnotationConfig {
            kind: "trendline".to_string(),
            color: [0.2, 0.3, 0.4],
            alpha: 0.5,
            width: 2.5,
            line_style: LineStyleConfig::Dashed,
            start_time: Some(1),
            start_price: Some(5.0),
            end_time: Some(2),
            end_price: Some(6.0),
            label: Some("note".to_string()),
            ..AnnotationConfig::default()
        },
    ];

    let config = KeroseneConfig {
        charts: vec![chart_with(annotations.clone())],
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");

    assert_eq!(decoded.charts[0].annotations, annotations);
    for cfg in &decoded.charts[0].annotations {
        assert!(
            Annotation::from_config(0, cfg).is_some(),
            "annotation should reconstruct: {cfg:?}"
        );
    }
}

#[test]
fn legacy_level_json_without_style_fields_loads() {
    // A blob saved before style fields existed (only the original keys).
    let legacy = r#"{"type":"level","color":[0.4,0.5,0.6],"price":100.0,"start_time":null,"start_price":null,"end_time":null,"end_price":null}"#;
    let cfg: AnnotationConfig = value_from_str(legacy, "legacy annotation should deserialize");
    assert_eq!(cfg.alpha, 1.0);
    assert_eq!(cfg.width, crate::annotations::DEFAULT_ANNOTATION_WIDTH);
    assert!(cfg.visible);
    assert!(!cfg.locked);
    let annotation = Annotation::from_config(0, &cfg).expect("legacy level should load");
    assert!(annotation.is_valid());
}

#[test]
fn default_style_level_omits_new_keys() {
    let config = chart_with(vec![AnnotationConfig {
        kind: "level".to_string(),
        color: [0.4, 0.5, 0.6],
        price: Some(100.0),
        ..AnnotationConfig::default()
    }]);
    let json = json_string(&config, "chart should serialize");
    assert!(!json.contains("\"alpha\""), "{json}");
    assert!(!json.contains("\"width\""), "{json}");
    assert!(!json.contains("\"visible\""), "{json}");
    assert!(!json.contains("\"line_style\""), "{json}");
    assert!(!json.contains("\"anchors\""), "{json}");
    assert!(!json.contains("\"locked\""), "{json}");
}

#[test]
fn unknown_line_style_loads_as_solid() {
    let json = r#"{"type":"level","color":[0.4,0.5,0.6],"price":100.0,"line_style":"scribble"}"#;
    let cfg: AnnotationConfig = value_from_str(json, "unknown line style should deserialize");

    assert_eq!(cfg.line_style, LineStyleConfig::Solid);
    let annotation = Annotation::from_config(0, &cfg).expect("annotation should reconstruct");
    assert!(annotation.is_valid());
}

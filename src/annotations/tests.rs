use super::*;

fn level_config(price: f64) -> AnnotationConfig {
    AnnotationConfig {
        kind: "level".into(),
        color: [0.25, 0.5, 0.75],
        price: Some(price),
        ..AnnotationConfig::default()
    }
}

fn trendline_config(start_price: f64, end_price: f64) -> AnnotationConfig {
    AnnotationConfig {
        kind: "trendline".into(),
        color: [0.25, 0.5, 0.75],
        start_time: Some(1_000),
        start_price: Some(start_price),
        end_time: Some(2_000),
        end_price: Some(end_price),
        ..AnnotationConfig::default()
    }
}

fn sample_style() -> AnnotationStyle {
    AnnotationStyle {
        color: Color::from_rgba(0.1, 0.2, 0.3, 0.4),
        width: 2.5,
        line_style: LineStyle::Dashed,
        label: Some("note".into()),
        locked: true,
        visible: false,
    }
}

#[test]
fn annotation_config_rejects_nonfinite_or_nonpositive_prices() {
    assert!(Annotation::from_config(0, &level_config(f64::NAN)).is_none());
    assert!(Annotation::from_config(0, &level_config(f64::INFINITY)).is_none());
    assert!(Annotation::from_config(0, &level_config(0.0)).is_none());
    assert!(Annotation::from_config(0, &level_config(-1.0)).is_none());
    assert!(Annotation::from_config(0, &trendline_config(10.0, f64::NAN)).is_none());
    assert!(Annotation::from_config(0, &trendline_config(f64::INFINITY, 10.0)).is_none());
}

#[test]
fn annotation_config_rejects_malformed_colors() {
    let mut config = level_config(100.0);
    config.color = [0.5, f32::NAN, 0.5];
    assert!(Annotation::from_config(0, &config).is_none());

    config.color = [0.5, 1.1, 0.5];
    assert!(Annotation::from_config(0, &config).is_none());

    config.color = [-0.1, 0.5, 0.5];
    assert!(Annotation::from_config(0, &config).is_none());
}

#[test]
fn valid_annotation_config_loads() {
    let annotation = Annotation::from_config(7, &trendline_config(10.0, 11.0))
        .expect("valid trendline should load");

    assert_eq!(annotation.id, 7);
    assert!(annotation.is_valid());
    assert!(matches!(
        annotation.kind,
        AnnotationKind::TrendLine {
            start: (1_000, 10.0),
            end: (2_000, 11.0)
        }
    ));
}

#[test]
fn runtime_annotation_validity_rejects_bad_prices_and_colors() {
    let valid = Annotation {
        id: 1,
        kind: AnnotationKind::HorizontalLevel { price: 10.0 },
        style: AnnotationStyle {
            color: DEFAULT_LEVEL_COLOR,
            ..AnnotationStyle::default()
        },
    };
    assert!(valid.is_valid());

    let invalid_price = Annotation {
        kind: AnnotationKind::HorizontalLevel { price: f64::NAN },
        ..valid.clone()
    };
    assert!(!invalid_price.is_valid());

    let invalid_color = Annotation {
        style: AnnotationStyle {
            color: Color {
                r: f32::INFINITY,
                g: 0.5,
                b: 0.5,
                a: 1.0,
            },
            ..AnnotationStyle::default()
        },
        ..valid
    };
    assert!(!invalid_color.is_valid());
}

// ---------------------------------------------------------------------------
// Forward-compatible persistence
// ---------------------------------------------------------------------------

#[test]
fn legacy_level_config_without_new_fields_loads_with_default_style() {
    // Mimics a JSON blob saved before style fields existed.
    let json = r#"{"type":"level","color":[0.4,0.5,0.6],"price":100.0,"start_time":null,"start_price":null,"end_time":null,"end_price":null}"#;
    let cfg: AnnotationConfig = serde_json::from_str(json).expect("legacy level must parse");
    let ann = Annotation::from_config(0, &cfg).expect("legacy level must load");
    assert_eq!(ann.style.width, DEFAULT_ANNOTATION_WIDTH);
    assert_eq!(ann.style.line_style, LineStyle::Solid);
    assert!(ann.style.visible);
    assert!(!ann.style.locked);
    // Alpha defaults to fully opaque when absent.
    assert_eq!(ann.style.color.a, 1.0);
}

#[test]
fn alpha_recovery_missing_present_and_invalid() {
    // Missing -> opaque.
    let mut cfg = level_config(50.0);
    assert_eq!(Annotation::from_config(0, &cfg).unwrap().style.color.a, 1.0);
    // Present -> preserved.
    cfg.alpha = 0.25;
    assert_eq!(
        Annotation::from_config(0, &cfg).unwrap().style.color.a,
        0.25
    );
    // Fully transparent is a legitimate stored value.
    cfg.alpha = 0.0;
    assert_eq!(Annotation::from_config(0, &cfg).unwrap().style.color.a, 0.0);
    // Invalid -> falls back to opaque (does not drop the annotation).
    cfg.alpha = f32::NAN;
    assert_eq!(Annotation::from_config(0, &cfg).unwrap().style.color.a, 1.0);
}

#[test]
fn default_style_annotation_serializes_without_new_keys() {
    // A level created with the default style must serialize to the legacy
    // shape: no alpha/width/line_style/visible/etc. keys.
    let ann = Annotation {
        id: 0,
        kind: AnnotationKind::HorizontalLevel { price: 100.0 },
        style: AnnotationStyle::default(),
    };
    let json = serde_json::to_string(&ann.to_config()).unwrap();
    assert!(!json.contains("alpha"), "{json}");
    assert!(!json.contains("width"), "{json}");
    assert!(!json.contains("line_style"), "{json}");
    assert!(!json.contains("visible"), "{json}");
    assert!(!json.contains("locked"), "{json}");
    assert!(!json.contains("anchors"), "{json}");
    assert!(json.contains("\"type\":\"level\""), "{json}");
}

#[test]
fn full_style_round_trips() {
    let ann = Annotation {
        id: 3,
        kind: AnnotationKind::TrendLine {
            start: (1_000, 10.0),
            end: (2_000, 20.0),
        },
        style: sample_style(),
    };
    let restored =
        Annotation::from_config(3, &ann.to_config()).expect("styled trendline must reload");
    assert_eq!(restored.style, ann.style);
    assert!(matches!(
        restored.kind,
        AnnotationKind::TrendLine {
            start: (1_000, 10.0),
            end: (2_000, 20.0)
        }
    ));
}

#[test]
fn new_kinds_round_trip() {
    let style = AnnotationStyle::default();
    let cases = vec![
        AnnotationKind::Ray {
            start: (1, 5.0),
            end: (2, 6.0),
        },
        AnnotationKind::ExtendedLine {
            start: (1, 5.0),
            end: (2, 6.0),
        },
        AnnotationKind::VerticalLine { time: 1_700 },
        AnnotationKind::Rectangle {
            a: (1, 5.0),
            b: (2, 6.0),
        },
        AnnotationKind::Measure {
            start: (1, 5.0),
            end: (2, 6.0),
        },
        AnnotationKind::Fib {
            kind: FibKind::Retracement,
            points: vec![(1, 5.0), (2, 6.0)],
        },
        AnnotationKind::Fib {
            kind: FibKind::Extension,
            points: vec![(1, 5.0), (2, 6.0), (3, 7.0)],
        },
    ];
    for kind in cases {
        let ann = Annotation {
            id: 1,
            kind: kind.clone(),
            style: style.clone(),
        };
        let restored = Annotation::from_config(1, &ann.to_config())
            .unwrap_or_else(|| panic!("kind must reload: {kind:?}"));
        assert_eq!(restored.kind, kind);
    }
}

#[test]
fn fib_with_wrong_anchor_count_is_rejected() {
    let mut cfg = AnnotationConfig {
        kind: "fib_extension".into(),
        color: [0.5, 0.5, 0.5],
        anchors: vec![AnchorConfig { t: 1, p: 5.0 }, AnchorConfig { t: 2, p: 6.0 }],
        ..AnnotationConfig::default()
    };
    // Extension requires 3 anchors.
    assert!(Annotation::from_config(0, &cfg).is_none());
    cfg.anchors.push(AnchorConfig { t: 3, p: 7.0 });
    assert!(Annotation::from_config(0, &cfg).is_some());
}

#[test]
fn unknown_kind_is_skipped() {
    let cfg = AnnotationConfig {
        kind: "supernova".into(),
        color: [0.5, 0.5, 0.5],
        ..AnnotationConfig::default()
    };
    assert!(Annotation::from_config(0, &cfg).is_none());
}

#[test]
fn translate_shifts_anchors_and_saturates_time() {
    let mut kind = AnnotationKind::TrendLine {
        start: (1_000, 10.0),
        end: (2_000, 20.0),
    };
    kind.translate(500, 1.0);
    assert!(matches!(
        kind,
        AnnotationKind::TrendLine {
            start: (1_500, p1),
            end: (2_500, p2)
        } if (p1 - 11.0).abs() < 1e-9 && (p2 - 21.0).abs() < 1e-9
    ));

    // Negative time shift saturates at zero rather than wrapping.
    let mut vline = AnnotationKind::VerticalLine { time: 100 };
    vline.translate(-1_000, 0.0);
    assert!(matches!(vline, AnnotationKind::VerticalLine { time: 0 }));

    // Horizontal level moves only in price.
    let mut level = AnnotationKind::HorizontalLevel { price: 10.0 };
    level.translate(9_999, -2.0);
    assert!(
        matches!(level, AnnotationKind::HorizontalLevel { price } if (price - 8.0).abs() < 1e-9)
    );
}

#[test]
fn set_anchor_updates_indexed_point() {
    let mut rect = AnnotationKind::Rectangle {
        a: (1, 5.0),
        b: (2, 6.0),
    };
    rect.set_anchor(1, (9, 99.0));
    assert!(
        matches!(rect, AnnotationKind::Rectangle { a: (1, _), b: (9, p) } if (p - 99.0).abs() < 1e-9)
    );
}

#[test]
fn fib_level_prices() {
    let start = (0, 100.0);
    let end = (0, 200.0);
    assert!((fib_retracement_price(start, end, 0.0) - 100.0).abs() < 1e-9);
    assert!((fib_retracement_price(start, end, 1.0) - 200.0).abs() < 1e-9);
    assert!((fib_retracement_price(start, end, 0.618) - 161.8).abs() < 1e-6);

    let a = (0, 100.0);
    let b = (0, 200.0);
    let c = (0, 150.0);
    assert!((fib_extension_price(a, b, c, 0.0) - 150.0).abs() < 1e-9);
    assert!((fib_extension_price(a, b, c, 1.0) - 250.0).abs() < 1e-9);
    assert!((fib_extension_price(a, b, c, 1.618) - 311.8).abs() < 1e-6);
}

#[test]
fn anchor_count_matches_tools() {
    assert_eq!(DrawingTool::HorizontalLevel.anchor_count(), 1);
    assert_eq!(DrawingTool::VerticalLine.anchor_count(), 1);
    assert_eq!(DrawingTool::TrendLine.anchor_count(), 2);
    assert_eq!(DrawingTool::Rectangle.anchor_count(), 2);
    assert_eq!(DrawingTool::FibRetracement.anchor_count(), 2);
    assert_eq!(DrawingTool::FibExtension.anchor_count(), 3);
    assert_eq!(DrawingTool::Select.anchor_count(), 0);
    assert_eq!(DrawingTool::Eraser.anchor_count(), 0);
    assert!(DrawingTool::TrendLine.is_shape());
    assert!(!DrawingTool::Select.is_shape());
}

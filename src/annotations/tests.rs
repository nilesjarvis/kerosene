use super::*;

fn level_config(price: f64) -> AnnotationConfig {
    AnnotationConfig {
        kind: "level".into(),
        color: [0.25, 0.5, 0.75],
        price: Some(price),
        start_time: None,
        start_price: None,
        end_time: None,
        end_price: None,
    }
}

fn trendline_config(start_price: f64, end_price: f64) -> AnnotationConfig {
    AnnotationConfig {
        kind: "trendline".into(),
        color: [0.25, 0.5, 0.75],
        price: None,
        start_time: Some(1_000),
        start_price: Some(start_price),
        end_time: Some(2_000),
        end_price: Some(end_price),
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
        color: DEFAULT_LEVEL_COLOR,
    };
    assert!(valid.is_valid());

    let invalid_price = Annotation {
        kind: AnnotationKind::HorizontalLevel { price: f64::NAN },
        ..valid.clone()
    };
    assert!(!invalid_price.is_valid());

    let invalid_color = Annotation {
        color: Color {
            r: f32::INFINITY,
            g: 0.5,
            b: 0.5,
            a: 1.0,
        },
        ..valid
    };
    assert!(!invalid_color.is_valid());
}

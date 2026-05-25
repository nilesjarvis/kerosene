use super::{SeriesLabelAnchor, legend_label, performance_label, stack_series_label_positions};

#[test]
fn legend_label_marks_empty_series_unavailable() {
    assert_eq!(legend_label("BTC", &[]), "BTC --");
}

#[test]
fn legend_label_formats_latest_percent() {
    assert_eq!(legend_label("BTC", &[(1.0, -2.5)]), "BTC -2.50%");
}

#[test]
fn performance_label_formats_signed_percent() {
    assert_eq!(performance_label(3.456), "+3.46%");
    assert_eq!(performance_label(-0.25), "-0.25%");
}

#[test]
fn series_label_stack_separates_nearby_labels() {
    let positions = stack_series_label_positions(
        vec![
            SeriesLabelAnchor { index: 0, y: 40.0 },
            SeriesLabelAnchor { index: 1, y: 42.0 },
            SeriesLabelAnchor { index: 2, y: 43.0 },
        ],
        240.0,
    );

    assert_eq!(positions[0], Some(40.0));
    assert_eq!(positions[1], Some(78.0));
    assert_eq!(positions[2], Some(116.0));
}

#[test]
fn series_label_stack_stays_inside_available_height_when_possible() {
    let positions = stack_series_label_positions(
        vec![
            SeriesLabelAnchor { index: 0, y: 92.0 },
            SeriesLabelAnchor { index: 1, y: 94.0 },
        ],
        100.0,
    );

    assert_eq!(positions[0], Some(43.0));
    assert_eq!(positions[1], Some(81.0));
}

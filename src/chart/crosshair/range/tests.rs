use super::*;

fn assert_near_f32(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-6,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn calculates_positive_range_measurement_label_and_bounds() {
    let measurement = calculate_range_measurement(100.0, 112.5, 120.0, 180.0, 80.0, 320.0, 220.0);

    assert!(measurement.is_up);
    assert_eq!(measurement.label, "+12.50% (+12.50)");
    assert_near_f32(measurement.anchor_y, 120.0);
    assert_near_f32(measurement.hover_y, 80.0);
    assert_near_f32(measurement.top, 80.0);
    assert_near_f32(measurement.bottom, 120.0);
    assert_near_f32(measurement.label_x, 190.0);
    assert_near_f32(measurement.label_y, 60.0);
}

#[test]
fn clamps_lines_and_keeps_label_inside_right_edge() {
    let measurement = calculate_range_measurement(100.0, 90.0, -40.0, 310.0, 260.0, 320.0, 220.0);

    assert!(!measurement.is_up);
    assert_eq!(measurement.label, "-10.00% (-10.00)");
    assert_near_f32(measurement.anchor_y, 0.0);
    assert_near_f32(measurement.hover_y, 220.0);
    assert_near_f32(measurement.top, 0.0);
    assert_near_f32(measurement.bottom, 220.0);
    assert!(measurement.label_x + measurement.label_width <= 316.0);
    assert_near_f32(measurement.label_y, 202.0);
}

#[test]
fn zero_anchor_price_uses_zero_percent() {
    let measurement = calculate_range_measurement(0.0, 25.0, 50.0, 20.0, 60.0, 240.0, 160.0);

    assert_eq!(measurement.label, "+0.00% (+25.00)");
}

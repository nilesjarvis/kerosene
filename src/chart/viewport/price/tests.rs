use super::*;

fn assert_close_f32(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.000_001,
        "expected {expected}, got {actual}"
    );
}

fn assert_close_f64(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 0.000_001,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn price_coordinate_conversion_round_trips_normal_axis() {
    let chart = CandlestickChart::new(1);

    let y = chart.price_to_y_with(60.0, 100.0, 50.0, 200.0);
    let price = chart.y_to_price_with(y, 100.0, 50.0, 200.0);

    assert_close_f32(y, 160.0);
    assert_close_f64(price, 60.0);
}

#[test]
fn price_coordinate_conversion_round_trips_inverted_axis() {
    let mut chart = CandlestickChart::new(1);
    chart.inverted = true;

    let y = chart.price_to_y_with(60.0, 100.0, 50.0, 200.0);
    let price = chart.y_to_price_with(y, 100.0, 50.0, 200.0);

    assert_close_f32(y, 40.0);
    assert_close_f64(price, 60.0);
}

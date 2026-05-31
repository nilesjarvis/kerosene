use super::*;

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-6,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn scale_maps_window_bounds_to_chart_edges() {
    let now = Instant::now();
    let data = VecDeque::from([
        (now, 10.0),
        (now - Duration::from_secs(150), 5.0),
        (now - Duration::from_secs(300), 1.0),
    ]);
    let scale = SpreadChartScale::new(&data, now, 120.0, 60.0);

    assert_close(scale.time_to_x(now - Duration::from_secs(300)), 0.0);
    assert_close(scale.time_to_x(now - Duration::from_secs(150)), 60.0);
    assert_close(scale.time_to_x(now), 120.0);
}

#[test]
fn scale_clamps_times_outside_window() {
    let now = Instant::now();
    let data = VecDeque::from([(now, 10.0)]);
    let scale = SpreadChartScale::new(&data, now, 120.0, 60.0);

    assert_close(scale.time_to_x(now - Duration::from_secs(400)), 0.0);
    assert_close(scale.time_to_x(now + Duration::from_secs(1)), 120.0);
}

#[test]
fn scale_uses_visible_spread_range_instead_of_zero_baseline() {
    let now = Instant::now();
    let data = VecDeque::from([
        (now, 100.50),
        (now - Duration::from_secs(150), 100.25),
        (now - Duration::from_secs(300), 100.00),
    ]);
    let scale = SpreadChartScale::new(&data, now, 120.0, 120.0);

    assert_close(scale.spread_to_y(100.00), 110.0);
    assert_close(scale.spread_to_y(100.25), 60.0);
    assert_close(scale.spread_to_y(100.50), 10.0);
}

#[test]
fn scale_centers_flat_spreads() {
    let now = Instant::now();
    let data = VecDeque::from([(now, 10.0), (now - Duration::from_secs(30), 10.0)]);
    let scale = SpreadChartScale::new(&data, now, 120.0, 110.0);

    assert_close(scale.spread_to_y(10.0), 55.0);
}

#[test]
fn rendered_points_are_oldest_to_newest() {
    let now = Instant::now();
    let data = VecDeque::from([
        (now, 3.0),
        (now - Duration::from_secs(60), 2.0),
        (now - Duration::from_secs(120), 1.0),
    ]);
    let scale = SpreadChartScale::new(&data, now, 300.0, 60.0);

    let points = rendered_spread_points(&data, &scale);

    assert_eq!(
        points.iter().map(|(_, spread)| *spread).collect::<Vec<_>>(),
        vec![1.0, 2.0, 3.0]
    );
}

#[test]
fn rendered_points_skip_non_finite_and_stale_spreads() {
    let now = Instant::now();
    let data = VecDeque::from([
        (now, 3.0),
        (now - Duration::from_secs(60), f64::NAN),
        (now - Duration::from_secs(120), 2.0),
        (now - Duration::from_secs(301), 1.0),
    ]);
    let scale = SpreadChartScale::new(&data, now, 300.0, 60.0);

    let points = rendered_spread_points(&data, &scale);

    assert_eq!(
        points.iter().map(|(_, spread)| *spread).collect::<Vec<_>>(),
        vec![2.0, 3.0]
    );
}

#[test]
fn closest_spread_point_uses_horizontal_distance() {
    let points = vec![
        (Point::new(10.0, 100.0), 1.0),
        (Point::new(30.0, 20.0), 2.0),
        (Point::new(60.0, 80.0), 3.0),
    ];

    let closest = closest_spread_point(&points, Point::new(34.0, 500.0));

    assert_eq!(closest, Some((Point::new(30.0, 20.0), 2.0)));
}

use super::*;

#[test]
fn point_to_segment_distance_projects_inside_segment() {
    let distance = point_to_segment_dist(5.0, 3.0, 0.0, 0.0, 10.0, 0.0);

    assert_eq!(distance, 3.0);
}

#[test]
fn point_to_segment_distance_clamps_to_nearest_endpoint() {
    let distance = point_to_segment_dist(13.0, 4.0, 0.0, 0.0, 10.0, 0.0);

    assert_eq!(distance, 5.0);
}

#[test]
fn clip_keeps_segment_fully_inside() {
    let clipped = clip_segment_to_rect(10.0, 10.0, 90.0, 90.0, 100.0, 100.0).unwrap();
    assert_eq!(clipped, (10.0, 10.0, 90.0, 90.0));
}

#[test]
fn clip_trims_segment_crossing_edges() {
    // Diagonal from outside-left to outside-right of a 100x100 box.
    let (x0, y0, x1, y1) = clip_segment_to_rect(-50.0, 50.0, 150.0, 50.0, 100.0, 100.0).unwrap();
    assert!((x0 - 0.0).abs() < 1e-3);
    assert!((x1 - 100.0).abs() < 1e-3);
    assert!((y0 - 50.0).abs() < 1e-3 && (y1 - 50.0).abs() < 1e-3);
}

#[test]
fn clip_rejects_fully_outside_segment() {
    assert!(clip_segment_to_rect(-20.0, -20.0, -5.0, -5.0, 100.0, 100.0).is_none());
}

#[test]
fn forward_ray_clips_to_right_edge() {
    // A ray heading right from (20,50) through (40,50) must reach the right edge.
    let (x0, y0, x1, y1) =
        extend_and_clip_line(20.0, 50.0, 40.0, 50.0, 100.0, 100.0, LineExtension::Forward).unwrap();
    assert!((x0 - 20.0).abs() < 1e-3, "ray starts at first anchor");
    assert!((x1 - 100.0).abs() < 1e-3, "ray reaches the right edge");
    assert!((y0 - 50.0).abs() < 1e-3 && (y1 - 50.0).abs() < 1e-3);
}

#[test]
fn extended_line_clips_both_edges() {
    let (x0, _y0, x1, _y1) =
        extend_and_clip_line(40.0, 50.0, 60.0, 50.0, 100.0, 100.0, LineExtension::Both).unwrap();
    let lo = x0.min(x1);
    let hi = x0.max(x1);
    assert!((lo - 0.0).abs() < 1e-3, "extends to left edge");
    assert!((hi - 100.0).abs() < 1e-3, "extends to right edge");
}

#[test]
fn extend_rejects_degenerate_direction() {
    assert!(
        extend_and_clip_line(10.0, 10.0, 10.0, 10.0, 100.0, 100.0, LineExtension::Forward)
            .is_none()
    );
}

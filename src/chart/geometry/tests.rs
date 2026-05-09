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

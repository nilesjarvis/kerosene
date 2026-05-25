use super::*;

#[test]
fn cutoff_with_baseline_inserts_prior_value_at_cutoff() {
    let points = vec![(1_000, 10.0), (2_000, 20.0), (3_000, 30.0)];

    assert_eq!(
        apply_cutoff_with_baseline(&points, 2_500),
        vec![(2_500, 20.0), (3_000, 30.0)]
    );
}

#[test]
fn cutoff_with_baseline_returns_cutoff_value_when_all_points_are_before_cutoff() {
    let points = vec![(1_000, 10.0), (2_000, 20.0)];

    assert_eq!(
        apply_cutoff_with_baseline(&points, 3_000),
        vec![(3_000, 20.0)]
    );
}

#[test]
fn cutoff_with_baseline_keeps_existing_cutoff_point_without_duplicate() {
    let points = vec![(1_000, 10.0), (2_000, 20.0), (3_000, 30.0)];

    assert_eq!(
        apply_cutoff_with_baseline(&points, 2_000),
        vec![(2_000, 20.0), (3_000, 30.0)]
    );
}

#[test]
fn cutoff_without_baseline_returns_future_points_only() {
    let points = vec![(2_000, 20.0), (3_000, 30.0)];

    assert_eq!(
        apply_cutoff_with_baseline(&points, 1_000),
        vec![(2_000, 20.0), (3_000, 30.0)]
    );
}

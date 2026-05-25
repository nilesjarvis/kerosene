use super::min_size::clamp_split_ratio;

#[test]
fn clamp_split_ratio_recovers_from_non_finite_ratio() {
    let ratio = clamp_split_ratio(f32::NAN, 500.0, 50.0, 50.0, false, false, 4.0);

    assert_eq!(ratio, 0.5);
}

#[test]
fn clamp_split_ratio_handles_invalid_axis_length() {
    let ratio = clamp_split_ratio(0.25, f32::NAN, 50.0, 50.0, false, false, 4.0);

    assert_eq!(ratio, 0.25);
}

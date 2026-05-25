use super::*;

#[test]
fn reset_view_clears_funding_axis_transform() {
    let mut state = ChartState {
        funding_y_scale: 0.25,
        funding_y_offset: 0.001,
        ..ChartState::default()
    };

    state.reset_view(42);

    assert_eq!(state.funding_y_scale, 1.0);
    assert_eq!(state.funding_y_offset, 0.0);
    assert_eq!(state.reset_epoch_seen, 42);
}

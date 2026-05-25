use super::*;

#[test]
fn chart_surface_widget_suffixes_distinguish_docked_charts() {
    assert_ne!(
        ChartSurfaceId::Docked(1).widget_suffix(),
        ChartSurfaceId::Docked(2).widget_suffix()
    );
}

#[test]
fn detached_chart_window_state_filters_nonfinite_config_values() {
    let config = crate::config::DetachedChartWindowConfig {
        chart_id: 7,
        width: f32::NAN,
        height: 10.0,
        x: Some(f32::NAN),
        y: Some(20.0),
    };

    let state = DetachedChartWindowState::from_config(&config);

    assert_eq!(
        state.width,
        crate::config::default_detached_chart_window_width()
    );
    assert_eq!(state.height, 320.0);
    assert_eq!(state.x, None);
    assert_eq!(state.y, Some(20.0));

    let mut state = DetachedChartWindowState::new(7);
    state.x = Some(f32::INFINITY);
    state.y = Some(30.0);

    let stored = state.to_config();

    assert_eq!(stored.x, None);
    assert_eq!(stored.y, Some(30.0));
}

#[test]
fn detached_chart_window_position_requires_finite_coordinates() {
    let mut state = DetachedChartWindowState::new(1);
    state.x = Some(10.0);
    state.y = Some(20.0);

    match state.position() {
        iced::window::Position::Specific(point) => {
            assert_eq!(point.x, 10.0);
            assert_eq!(point.y, 20.0);
        }
        _ => panic!("expected specific position"),
    }

    state.y = Some(f32::NAN);

    assert!(matches!(state.position(), iced::window::Position::Centered));
}

use super::*;

#[test]
fn onboarding_phase_advances_and_wraps_without_skipping_a_step() {
    let (mut terminal, _) = TradingTerminal::boot();
    assert_eq!(terminal.onboarding_phase, 0.0);

    for _ in 0..2_000 {
        let before = terminal.onboarding_phase;
        terminal.advance_onboarding_phase();
        let after = terminal.onboarding_phase;
        assert!((0.0..ONBOARDING_PHASE_PERIOD).contains(&after));
        let delta = (after - before).rem_euclid(ONBOARDING_PHASE_PERIOD);
        assert!((delta - ONBOARDING_PHASE_STEP).abs() < 1e-5);
    }
}

#[test]
fn gradient_moves_and_returns_to_the_same_geometry_and_colors_at_wrap() {
    for theme in [Theme::Dark, Theme::Light] {
        let palette = theme.palette();
        for size in [Size::new(320.0, 480.0), Size::new(1920.0, 1080.0)] {
            let gradient =
                |phase| onboarding_gradient(size, phase, palette.background, palette.primary);
            let start = gradient(0.0);
            assert_ne!(start, gradient(ONBOARDING_PHASE_PERIOD / 4.0));

            let end = gradient(ONBOARDING_PHASE_PERIOD);
            assert!(start.start.distance(end.start) < 0.001);
            assert!(start.end.distance(end.end) < 0.001);
            assert_eq!(start.stops, end.stops);
        }
    }
}

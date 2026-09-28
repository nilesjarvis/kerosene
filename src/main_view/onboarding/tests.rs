use super::*;

#[test]
fn onboarding_phase_advances_then_wraps_within_period() {
    let (mut terminal, _) = TradingTerminal::boot();
    assert_eq!(terminal.onboarding_phase, 0.0);

    terminal.advance_onboarding_phase();
    assert!((terminal.onboarding_phase - ONBOARDING_PHASE_STEP).abs() < 1e-4);

    // Drive well past the period; the phase must stay bounded and only ever move
    // forward by one step or wrap back toward zero.
    for _ in 0..5_000 {
        let before = terminal.onboarding_phase;
        terminal.advance_onboarding_phase();
        let after = terminal.onboarding_phase;
        assert!(
            (0.0..ONBOARDING_PHASE_PERIOD).contains(&after),
            "phase {after} escaped [0, period)"
        );
        let delta = after - before;
        assert!(
            (delta - ONBOARDING_PHASE_STEP).abs() < 1e-3 || delta < 0.0,
            "unexpected phase delta {delta}"
        );
    }
}

#[test]
fn onboarding_phase_period_keeps_animation_terms_seamless() {
    // The period must be a whole multiple of TAU so every sine/cosine term in
    // the canvases lands on a full period when the phase wraps.
    let multiples = ONBOARDING_PHASE_PERIOD / std::f32::consts::TAU;
    assert!((multiples - multiples.round()).abs() < 1e-3);

    // Every non-unit phase coefficient used by the onboarding canvases must turn
    // into a whole number when scaled by that multiple, or the wrap would still
    // produce a visible jump. Update this list if a coefficient changes.
    for coeff in [0.8_f32, 0.72, 1.36, 0.9, 0.7, 0.43] {
        let scaled = coeff * multiples;
        assert!(
            (scaled - scaled.round()).abs() < 1e-2,
            "coefficient {coeff} is not seamless at the phase period"
        );
    }
}

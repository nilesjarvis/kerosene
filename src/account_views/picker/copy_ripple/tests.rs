use super::{
    RIPPLE_DURATION, RippleState, ease_out_cubic, max_ripple_radius, ripple_alpha, ripple_progress,
    ripple_radius,
};
use iced::{Point, Size};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Copy Ripple Geometry Tests
// ---------------------------------------------------------------------------

#[test]
fn max_ripple_radius_reaches_the_farthest_corner() {
    let size = Size::new(100.0, 40.0);

    // Click in a corner: the farthest corner is the diagonal opposite.
    let from_corner = max_ripple_radius(size, Point::new(0.0, 0.0));
    assert!((from_corner - (100.0_f32.hypot(40.0))).abs() < 1e-3);

    // Click in the center: half the diagonal in each direction.
    let from_center = max_ripple_radius(size, Point::new(50.0, 20.0));
    assert!((from_center - (50.0_f32.hypot(20.0))).abs() < 1e-3);

    // A corner click should always cover at least as much as a center click.
    assert!(from_corner > from_center);
}

#[test]
fn ripple_radius_grows_from_zero_to_full_coverage() {
    let size = Size::new(80.0, 30.0);
    let origin = Point::new(20.0, 15.0);
    let full = max_ripple_radius(size, origin);

    assert_eq!(ripple_radius(size, origin, 0.0), 0.0);
    assert!((ripple_radius(size, origin, 1.0) - full).abs() < 1e-3);

    // Monotonically increasing as the animation plays out.
    let early = ripple_radius(size, origin, 0.25);
    let mid = ripple_radius(size, origin, 0.5);
    assert!(early > 0.0);
    assert!(mid > early);
    assert!(mid < full);
}

#[test]
fn ripple_alpha_peaks_at_the_start_and_fades_to_zero() {
    let start = ripple_alpha(0.0);
    let mid = ripple_alpha(0.5);
    let end = ripple_alpha(1.0);

    assert!(start > mid);
    assert!(mid > end);
    assert_eq!(end, 0.0);
    assert!(start > 0.0);
}

#[test]
fn ripple_progress_uses_elapsed_redraw_time() {
    assert_eq!(ripple_progress(Duration::ZERO), 0.0);
    assert_eq!(ripple_progress(RIPPLE_DURATION / 2), 0.5);
    assert_eq!(ripple_progress(RIPPLE_DURATION * 2), 1.0);
}

#[test]
fn ripple_state_initializes_from_redraw_clock() {
    let mut state = RippleState::default();
    let origin = Point::new(8.0, 12.0);
    state.start(origin);

    let ripple = state.ripple.expect("ripple starts after press");
    assert_eq!(ripple.origin, origin);
    assert_eq!(ripple.started_at, None);
    assert_eq!(ripple.progress, 0.0);

    let now = Instant::now();
    assert!(state.advance(now));
    let ripple = state.ripple.expect("ripple is active on first redraw");
    assert_eq!(ripple.started_at, Some(now));
    assert_eq!(ripple.progress, 0.0);

    assert!(state.advance(now + RIPPLE_DURATION / 2));
    let ripple = state.ripple.expect("ripple is active before duration ends");
    assert_eq!(ripple.started_at, Some(now));
    assert_eq!(ripple.progress, 0.5);

    assert!(!state.advance(now + RIPPLE_DURATION));
    assert!(state.ripple.is_none());
}

#[test]
fn ease_out_cubic_is_clamped_and_front_loaded() {
    assert_eq!(ease_out_cubic(0.0), 0.0);
    assert_eq!(ease_out_cubic(1.0), 1.0);
    assert_eq!(ease_out_cubic(-1.0), 0.0);
    assert_eq!(ease_out_cubic(2.0), 1.0);
    assert!(ease_out_cubic(0.25) > 0.25);
}

use super::*;

#[test]
fn randomized_sizes_never_overshoot_target() {
    let now = Instant::now();
    let mut twap = test_twap_order(now, 10.0, true, 10);
    let mut total = 0.0;
    while twap.slices_attempted < twap.slice_count {
        let slice = next_slice(&mut twap, "slice should calculate");
        assert!(slice > 0.0);
        assert!(slice <= twap.remaining_size);
        total += slice;
        twap.remaining_size = (twap.remaining_size - slice).max(0.0);
        twap.slices_attempted += 1;
    }
    assert!(total <= 10.0 + 1e-9);
    assert!(twap.remaining_size <= f64::EPSILON);
}

#[test]
fn next_slice_size_rejects_nonpositive_or_nonfinite_remaining_size() {
    let now = Instant::now();
    for remaining_size in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut twap = test_twap_order(now, 10.0, false, 2);
        twap.remaining_size = remaining_size;
        assert_eq!(twap.next_slice_size(), None);
    }
}

#[test]
fn skipped_slices_roll_size_forward() {
    let now = Instant::now();
    let mut twap = test_twap_order(now, 9.0, false, 3);
    let first = next_slice(&mut twap, "first slice");
    assert_eq!(first, 3.0);
    twap.slices_attempted += 1;
    let second = next_slice(&mut twap, "rolled slice");
    assert_eq!(second, 4.5);
}

#[test]
fn slice_size_quantization_respects_asset_precision_without_rounding_up() {
    assert_eq!(quantize_twap_slice_size(1.239, 2.0, 2), Some(1.23));
    assert_eq!(quantize_twap_slice_size(1.239, 1.2, 2), Some(1.2));
    assert_eq!(quantize_twap_slice_size(0.9, 0.9, 0), None);
    assert_eq!(quantize_twap_slice_size(1.9, 2.0, 0), Some(1.0));
}

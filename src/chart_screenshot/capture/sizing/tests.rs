use super::*;

#[test]
fn exports_are_sharp_for_small_and_large_panes() {
    for (logical, expected) in [
        (Size::new(320.0, 180.0), Size::new(1920, 1080)),
        (Size::new(500.0, 300.0), Size::new(1920, 1152)),
        (Size::new(1280.0, 720.0), Size::new(3840, 2160)),
        (Size::new(720.0, 1280.0), Size::new(2160, 3840)),
    ] {
        let resolution = ExportResolution::new(logical).expect("valid size");
        assert_eq!(resolution.size, expected);
        assert!(resolution.scale_factor >= 3.0);
    }
}

#[test]
fn fractional_layout_is_preserved_to_within_one_output_pixel() {
    let logical = Size::new(1000.25, 600.75);
    let resolution = ExportResolution::new(logical).expect("fractional bounds");
    assert_eq!(resolution.scale_factor, 3.0);
    assert_eq!(resolution.size, Size::new(3000, 1802));
    for (logical_edge, pixels) in [
        (logical.width, resolution.size.width),
        (logical.height, resolution.size.height),
    ] {
        assert!((logical_edge * resolution.scale_factor - pixels as f32).abs() < 1.0);
    }
}

#[test]
fn limits_apply_even_when_the_source_already_exceeds_them() {
    for logical in [
        Size::new(4000.0, 100.0),
        Size::new(100.0, 4000.0),
        Size::new(16000.0, 9000.0),
        Size::new(1.0, 100000.0),
        Size::new(100000.0, 1.0),
        Size::new(f32::MAX, f32::MAX),
    ] {
        let resolution = ExportResolution::new(logical).expect("bounded export");
        assert!(resolution.size.width > 0 && resolution.size.width <= MAX_EXPORT_EDGE);
        assert!(resolution.size.height > 0 && resolution.size.height <= MAX_EXPORT_EDGE);
        assert!(
            u64::from(resolution.size.width) * u64::from(resolution.size.height)
                <= MAX_EXPORT_PIXELS
        );
        assert!(resolution.scale_factor.is_finite() && resolution.scale_factor > 0.0);
    }
}

#[test]
fn invalid_or_unrenderable_bounds_are_rejected() {
    for edge in [0.0, -1.0, 0.5, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(ExportResolution::new(Size::new(edge, 800.0)).is_err());
        assert!(ExportResolution::new(Size::new(800.0, edge)).is_err());
    }
}

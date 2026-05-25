use super::*;

#[test]
fn chart_screenshot_export_dimensions_upscale_small_charts() {
    assert_eq!(
        chart_screenshot_export_dimensions(320, 180),
        Some((1280, 720))
    );
    assert_eq!(
        chart_screenshot_export_dimensions(500, 300),
        Some((1280, 768))
    );
}

#[test]
fn chart_screenshot_export_dimensions_preserve_large_charts() {
    assert_eq!(chart_screenshot_export_dimensions(1600, 900), None);
    assert_eq!(chart_screenshot_export_dimensions(1280, 720), None);
}

#[test]
fn chart_screenshot_export_dimensions_cap_extreme_shapes() {
    let (width, height) = dimensions_or_panic(
        chart_screenshot_export_dimensions(4000, 100),
        "extreme chart still upscales",
    );

    assert_eq!(width, CHART_SCREENSHOT_MAX_EXPORT_EDGE);
    assert_eq!(height, 205);
}

#[test]
fn chart_screenshot_export_size_uses_logical_bounds() {
    let size = export_size_or_panic(
        chart_screenshot_export_size(Rectangle {
            x: 10.0,
            y: 20.0,
            width: 500.2,
            height: 300.4,
        }),
        "export size",
    );

    assert_eq!(size, (1280, 768));
}

#[test]
fn chart_screenshot_export_size_rejects_invalid_bounds() {
    let err = error_or_panic(
        chart_screenshot_export_size(Rectangle {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 20.0,
        }),
        "invalid bounds",
    );

    assert!(err.contains("invalid chart bounds"));
}

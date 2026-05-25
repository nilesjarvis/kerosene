use iced::Rectangle;

const CHART_SCREENSHOT_MIN_EXPORT_WIDTH: u32 = 1280;
const CHART_SCREENSHOT_MIN_EXPORT_HEIGHT: u32 = 720;
pub(in crate::chart_screenshot) const CHART_SCREENSHOT_MAX_EXPORT_EDGE: u32 = 8192;
const CHART_SCREENSHOT_MAX_EXPORT_PIXELS: u64 = 12_582_912;

// ---------------------------------------------------------------------------
// Export Sizing
// ---------------------------------------------------------------------------

pub(in crate::chart_screenshot) fn chart_screenshot_export_size(
    logical_bounds: Rectangle,
) -> Result<(u32, u32), String> {
    if !logical_bounds.width.is_finite()
        || !logical_bounds.height.is_finite()
        || logical_bounds.width <= 0.0
        || logical_bounds.height <= 0.0
    {
        return Err("invalid chart bounds".to_string());
    }

    let width = logical_bounds.width.round().max(1.0) as u32;
    let height = logical_bounds.height.round().max(1.0) as u32;
    Ok(chart_screenshot_export_dimensions(width, height).unwrap_or((width, height)))
}

pub(in crate::chart_screenshot) fn chart_screenshot_export_dimensions(
    width: u32,
    height: u32,
) -> Option<(u32, u32)> {
    if width == 0 || height == 0 {
        return None;
    }

    let width_scale = CHART_SCREENSHOT_MIN_EXPORT_WIDTH as f64 / width as f64;
    let height_scale = CHART_SCREENSHOT_MIN_EXPORT_HEIGHT as f64 / height as f64;
    let requested_scale = width_scale.max(height_scale).max(1.0);
    if requested_scale <= 1.0 {
        return None;
    }

    let edge_scale = CHART_SCREENSHOT_MAX_EXPORT_EDGE as f64 / width.max(height) as f64;
    let pixel_scale =
        (CHART_SCREENSHOT_MAX_EXPORT_PIXELS as f64 / (width as f64 * height as f64)).sqrt();
    let max_scale = edge_scale.max(1.0).min(pixel_scale.max(1.0));
    let scale = requested_scale.min(max_scale);
    if scale <= 1.0 {
        return None;
    }

    let target_width = ((width as f64 * scale).round() as u32).max(width);
    let target_height = ((height as f64 * scale).round() as u32).max(height);
    if target_width == width && target_height == height {
        None
    } else {
        Some((target_width, target_height))
    }
}

use iced::Size;

const EXPORT_SCALE: f64 = 3.0;
const MIN_EXPORT_EDGE: f64 = 1920.0;
const MAX_EXPORT_EDGE: u32 = 8192;
const MAX_EXPORT_PIXELS: u64 = 12_582_912;

/// Pixel density is independent of layout. Never substitute these physical
/// dimensions for the canvas's logical dimensions when drawing the chart.
#[derive(Debug, Clone, Copy)]
pub(super) struct ExportResolution {
    pub(super) size: Size<u32>,
    pub(super) scale_factor: f32,
}

impl ExportResolution {
    pub(super) fn new(logical_size: Size) -> Result<Self, String> {
        let width = f64::from(logical_size.width);
        let height = f64::from(logical_size.height);
        if !width.is_finite() || !height.is_finite() || width < 1.0 || height < 1.0 {
            return Err("invalid chart bounds".to_string());
        }

        let longest = width.max(height);
        let requested_scale = EXPORT_SCALE.max(MIN_EXPORT_EDGE / longest);
        let edge_limit = f64::from(MAX_EXPORT_EDGE) / longest;
        let pixel_limit = (MAX_EXPORT_PIXELS as f64 / (width * height)).sqrt();
        let scale = requested_scale.min(edge_limit).min(pixel_limit);

        // Round down only at the final pixel boundary. Even an oversized source
        // is capped, and fractional logical bounds are never rounded/reflowed.
        Ok(Self {
            size: Size::new(
                (width * scale).floor().max(1.0) as u32,
                (height * scale).floor().max(1.0) as u32,
            ),
            scale_factor: scale as f32,
        })
    }
}

#[cfg(test)]
mod tests;

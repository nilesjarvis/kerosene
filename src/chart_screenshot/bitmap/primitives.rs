use iced::Color;
use image::codecs::png::PngEncoder;
use image::{ColorType, ImageEncoder};

// ---------------------------------------------------------------------------
// Bitmap Primitives
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(crate) struct Rect {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PixelPoint {
    pub(crate) x: u32,
    pub(crate) y: u32,
}

pub(crate) fn encode_png_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let expected_len = width as usize * height as usize * 4;
    if rgba.len() != expected_len {
        return Err("captured image buffer had an unexpected size".to_string());
    }

    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(rgba, width, height, ColorType::Rgba8.into())
        .map_err(|err| err.to_string())?;
    Ok(png)
}

pub(crate) fn color_to_rgba(color: Color, alpha: u8) -> [u8; 4] {
    [
        color_to_u8(color.r),
        color_to_u8(color.g),
        color_to_u8(color.b),
        alpha,
    ]
}

fn color_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub(crate) fn fill_rect(rgba: &mut [u8], width: u32, height: u32, rect: Rect, color: [u8; 4]) {
    let max_x = rect.x.saturating_add(rect.width).min(width);
    let max_y = rect.y.saturating_add(rect.height).min(height);
    for y in rect.y..max_y {
        for x in rect.x..max_x {
            blend_pixel(rgba, width, x, y, color);
        }
    }
}

pub(crate) fn stroke_rect(rgba: &mut [u8], width: u32, height: u32, rect: Rect, color: [u8; 4]) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }

    let right = rect.x.saturating_add(rect.width).saturating_sub(1);
    let bottom = rect.y.saturating_add(rect.height).saturating_sub(1);
    for x in rect.x..=right.min(width.saturating_sub(1)) {
        if rect.y < height {
            blend_pixel(rgba, width, x, rect.y, color);
        }
        if bottom < height {
            blend_pixel(rgba, width, x, bottom, color);
        }
    }
    for y in rect.y..=bottom.min(height.saturating_sub(1)) {
        if rect.x < width {
            blend_pixel(rgba, width, rect.x, y, color);
        }
        if right < width {
            blend_pixel(rgba, width, right, y, color);
        }
    }
}

fn blend_pixel(rgba: &mut [u8], width: u32, x: u32, y: u32, color: [u8; 4]) {
    let idx = (y as usize * width as usize + x as usize) * 4;
    if idx + 3 >= rgba.len() {
        return;
    }

    let alpha = color[3] as u16;
    let inv_alpha = 255 - alpha;
    for channel in 0..3 {
        rgba[idx + channel] =
            ((color[channel] as u16 * alpha + rgba[idx + channel] as u16 * inv_alpha) / 255) as u8;
    }
    rgba[idx + 3] = 255;
}

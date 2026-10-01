use image::codecs::png::PngEncoder;
use image::{ColorType, ImageEncoder};

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

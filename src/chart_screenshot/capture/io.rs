use super::super::ChartScreenshotState;

use arboard::{Clipboard, ImageData};
use chrono::{DateTime, Local};
use std::borrow::Cow;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Export IO
// ---------------------------------------------------------------------------

pub(in crate::chart_screenshot) fn copy_chart_screenshot_to_clipboard(
    state: ChartScreenshotState,
) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|err| err.to_string())?;
    clipboard
        .set_image(ImageData {
            width: state.width as usize,
            height: state.height as usize,
            bytes: Cow::Owned(state.rgba.as_ref().to_vec()),
        })
        .map_err(|err| err.to_string())
}

pub(in crate::chart_screenshot) async fn save_chart_screenshot_png(
    state: ChartScreenshotState,
) -> Result<Option<PathBuf>, String> {
    let path = rfd::AsyncFileDialog::new()
        .add_filter("PNG image", &["png"])
        .set_file_name(state.default_filename)
        .save_file()
        .await;

    let Some(path) = path else {
        return Ok(None);
    };

    std::fs::write(path.path(), state.png.as_ref()).map_err(|err| err.to_string())?;
    Ok(Some(path.path().to_path_buf()))
}

pub(in crate::chart_screenshot) fn chart_screenshot_filename(
    symbol: &str,
    timeframe: &str,
    captured_at: DateTime<Local>,
) -> String {
    format!(
        "kerosene-{}-{}-{}.png",
        sanitize_filename_part(symbol),
        sanitize_filename_part(timeframe),
        captured_at.format("%Y%m%d-%H%M%S")
    )
}

fn sanitize_filename_part(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "chart".to_string()
    } else {
        trimmed.to_string()
    }
}

use super::PnlCardImage;

use arboard::{Clipboard, ImageData};
use std::borrow::Cow;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Export Output
// ---------------------------------------------------------------------------

pub(in crate::pnl_card) fn copy_pnl_card_to_clipboard(image: PnlCardImage) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|err| err.to_string())?;
    clipboard
        .set_image(ImageData {
            width: image.width as usize,
            height: image.height as usize,
            bytes: Cow::Owned(image.rgba),
        })
        .map_err(|err| err.to_string())
}

pub(in crate::pnl_card) async fn save_pnl_card_png(
    image: PnlCardImage,
) -> Result<Option<PathBuf>, String> {
    let path = rfd::AsyncFileDialog::new()
        .add_filter("PNG image", &["png"])
        .set_file_name(image.default_filename)
        .save_file()
        .await;

    let Some(path) = path else {
        return Ok(None);
    };

    std::fs::write(path.path(), &image.png).map_err(|err| err.to_string())?;
    Ok(Some(path.path().to_path_buf()))
}

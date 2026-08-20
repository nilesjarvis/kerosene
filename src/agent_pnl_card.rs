use base64::Engine;
use iced::widget::image::Handle as ImageHandle;
use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader, Limits};
use std::fmt;
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use zeroize::Zeroizing;

const MAX_SOURCE_BYTES: u64 = 12 * 1024 * 1024;
const MAX_ENCODED_BYTES: usize = 8 * 1024 * 1024;
const MAX_SOURCE_DIMENSION: u32 = 12_000;
const MAX_MODEL_DIMENSION: u32 = 2_000;
const MAX_DECODE_ALLOC_BYTES: u64 = 192 * 1024 * 1024;
const MAX_FILE_LABEL_CHARS: usize = 80;

// ---------------------------------------------------------------------------
// Assistant P&L Card Attachments
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(crate) struct AgentPnlCardAttachment {
    pub(crate) file_label: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) png: Arc<[u8]>,
    pub(crate) preview_handle: ImageHandle,
}

impl AgentPnlCardAttachment {
    pub(crate) fn prompt_image(&self) -> AgentPromptImage {
        AgentPromptImage {
            mime_type: "image/png",
            data: Zeroizing::new(base64::engine::general_purpose::STANDARD.encode(&self.png)),
        }
    }
}

impl fmt::Debug for AgentPnlCardAttachment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AgentPnlCardAttachment")
            .field("file_label", &"<redacted>")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("png_len", &self.png.len())
            .finish()
    }
}

#[derive(Clone)]
pub(crate) struct AgentPromptImage {
    pub(crate) mime_type: &'static str,
    pub(crate) data: Zeroizing<String>,
}

impl fmt::Debug for AgentPromptImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AgentPromptImage(<redacted>)")
    }
}

#[derive(Clone)]
pub(crate) struct AgentPnlCardPath(PathBuf);

impl AgentPnlCardPath {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub(crate) fn into_path_buf(self) -> PathBuf {
        self.0
    }
}

impl fmt::Debug for AgentPnlCardPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AgentPnlCardPath(<redacted>)")
    }
}

#[derive(Clone)]
pub(crate) struct AgentPnlCardLoadResult(Result<Option<AgentPnlCardAttachment>, String>);

impl AgentPnlCardLoadResult {
    pub(crate) fn into_result(self) -> Result<Option<AgentPnlCardAttachment>, String> {
        self.0
    }
}

impl From<Result<Option<AgentPnlCardAttachment>, String>> for AgentPnlCardLoadResult {
    fn from(value: Result<Option<AgentPnlCardAttachment>, String>) -> Self {
        Self(value)
    }
}

impl fmt::Debug for AgentPnlCardLoadResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Ok(Some(_)) => f.write_str("AgentPnlCardLoadResult(Ok(Some(<redacted>)))"),
            Ok(None) => f.write_str("AgentPnlCardLoadResult(Ok(None))"),
            Err(_) => f.write_str("AgentPnlCardLoadResult(Err(<redacted>))"),
        }
    }
}

pub(crate) async fn choose_agent_pnl_card() -> Result<Option<AgentPnlCardAttachment>, String> {
    let selected = rfd::AsyncFileDialog::new()
        .add_filter("P&L card image", &["png", "jpg", "jpeg", "webp"])
        .pick_file()
        .await;
    let Some(selected) = selected else {
        return Ok(None);
    };
    load_agent_pnl_card(selected.path().to_path_buf()).await
}

pub(crate) async fn load_agent_pnl_card(
    path: PathBuf,
) -> Result<Option<AgentPnlCardAttachment>, String> {
    prepare_agent_pnl_card(&path).map(Some)
}

fn prepare_agent_pnl_card(path: &Path) -> Result<AgentPnlCardAttachment, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("Could not inspect the selected P&L card: {error}"))?;
    if !metadata.is_file() {
        return Err("The dropped item is not an image file.".to_string());
    }
    if metadata.len() == 0 {
        return Err("The selected P&L card is empty.".to_string());
    }
    if metadata.len() > MAX_SOURCE_BYTES {
        return Err(format!(
            "The selected P&L card is too large (maximum {} MiB).",
            MAX_SOURCE_BYTES / 1024 / 1024
        ));
    }

    let file = std::fs::File::open(path)
        .map_err(|error| format!("Could not open the selected P&L card: {error}"))?;
    let mut reader = ImageReader::new(BufReader::new(file))
        .with_guessed_format()
        .map_err(|error| format!("Could not identify the selected image: {error}"))?;
    let format = reader
        .format()
        .ok_or_else(|| "Use a PNG, JPEG, or WebP P&L card.".to_string())?;
    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP
    ) {
        return Err("Use a PNG, JPEG, or WebP P&L card.".to_string());
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_SOURCE_DIMENSION);
    limits.max_image_height = Some(MAX_SOURCE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOC_BYTES);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|error| format!("Could not decode the selected P&L card: {error}"))?;
    let prepared = resize_for_model(decoded);
    let (width, height) = prepared.dimensions();
    let mut png = Vec::new();
    prepared
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|error| format!("Could not prepare the P&L card for analysis: {error}"))?;
    if png.len() > MAX_ENCODED_BYTES {
        return Err(format!(
            "The prepared P&L card is too large (maximum {} MiB).",
            MAX_ENCODED_BYTES / 1024 / 1024
        ));
    }

    let file_label = bounded_file_label(path);
    let preview_handle = ImageHandle::from_bytes(png.clone());
    Ok(AgentPnlCardAttachment {
        file_label,
        width,
        height,
        png: Arc::from(png),
        preview_handle,
    })
}

fn resize_for_model(image: DynamicImage) -> DynamicImage {
    let (width, height) = image.dimensions();
    if width <= MAX_MODEL_DIMENSION && height <= MAX_MODEL_DIMENSION {
        image
    } else {
        image.thumbnail(MAX_MODEL_DIMENSION, MAX_MODEL_DIMENSION)
    }
}

fn bounded_file_label(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("P&L card");
    let mut chars = name.chars();
    let mut bounded = chars
        .by_ref()
        .take(MAX_FILE_LABEL_CHARS)
        .collect::<String>();
    if chars.next().is_some() {
        let _ = bounded.pop();
        bounded.push('…');
    }
    bounded
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    #[test]
    fn preparation_normalizes_and_bounds_supported_images() {
        let directory =
            std::env::temp_dir().join(format!("kerosene-agent-card-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("temporary directory");
        let path = directory.join("social-card.png");
        let source = ImageBuffer::from_pixel(2_400, 1_200, Rgba([20_u8, 30, 40, 255]));
        DynamicImage::ImageRgba8(source)
            .save_with_format(&path, ImageFormat::Png)
            .expect("fixture should save");

        let attachment = prepare_agent_pnl_card(&path).expect("card should prepare");

        assert_eq!((attachment.width, attachment.height), (2_000, 1_000));
        assert!(attachment.png.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert_eq!(attachment.file_label, "social-card.png");
        assert_eq!(attachment.prompt_image().mime_type, "image/png");

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn path_and_load_result_debug_output_redacts_file_details() {
        let path = AgentPnlCardPath::new(PathBuf::from("/private/alice/card.png"));
        assert_eq!(format!("{path:?}"), "AgentPnlCardPath(<redacted>)");

        let result: AgentPnlCardLoadResult = Err("/private/alice/card.png".to_string()).into();
        let rendered = format!("{result:?}");
        assert!(!rendered.contains("alice"));
        assert!(!rendered.contains("card.png"));
    }
}

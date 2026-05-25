use chrono::{DateTime, Local};
use iced::widget::image::Handle as ImageHandle;
use std::fmt;
use std::sync::Arc;

mod bitmap;
mod capture;
mod label;
mod update;
mod view;

pub(crate) use bitmap::{
    PixelPoint, Rect, bitmap_text_width, color_to_rgba, draw_bitmap_text, encode_png_rgba,
    fill_rect,
};
#[cfg(test)]
use capture::*;
#[cfg(test)]
use label::*;
#[cfg(test)]
use update::chart_for_screenshot_export;

// ---------------------------------------------------------------------------
// Screenshot State
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(crate) struct ChartScreenshotState {
    pub(crate) symbol: String,
    pub(crate) timeframe: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Arc<[u8]>,
    pub(crate) png: Arc<[u8]>,
    pub(crate) preview_handle: ImageHandle,
    pub(crate) captured_at: DateTime<Local>,
    pub(crate) default_filename: String,
}

impl fmt::Debug for ChartScreenshotState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChartScreenshotState")
            .field("symbol", &self.symbol)
            .field("timeframe", &self.timeframe)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("rgba_len", &self.rgba.len())
            .field("png_len", &self.png.len())
            .field("captured_at", &self.captured_at)
            .field("default_filename", &self.default_filename)
            .finish()
    }
}

#[cfg(test)]
mod tests;

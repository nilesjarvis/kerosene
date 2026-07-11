use crate::chart_state::{ChartId, ChartSurfaceId};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChartScreenshotCaptureRequest {
    request_id: u64,
    chart_id: ChartId,
    chart_instance_generation: u64,
    surface_id: ChartSurfaceId,
}

impl ChartScreenshotCaptureRequest {
    pub(crate) fn new(
        request_id: u64,
        chart_id: ChartId,
        chart_instance_generation: u64,
        surface_id: ChartSurfaceId,
    ) -> Self {
        Self {
            request_id,
            chart_id,
            chart_instance_generation,
            surface_id,
        }
    }

    #[cfg(test)]
    pub(crate) fn request_id(self) -> u64 {
        self.request_id
    }

    pub(crate) fn chart_id(self) -> ChartId {
        self.chart_id
    }

    pub(crate) fn chart_instance_generation(self) -> u64 {
        self.chart_instance_generation
    }

    pub(crate) fn surface_id(self) -> ChartSurfaceId {
        self.surface_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChartScreenshotCapturePhase {
    AwaitingBounds,
    Rendering,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChartScreenshotPendingCapture {
    request: ChartScreenshotCaptureRequest,
    phase: ChartScreenshotCapturePhase,
}

impl ChartScreenshotPendingCapture {
    pub(crate) fn awaiting_bounds(request: ChartScreenshotCaptureRequest) -> Self {
        Self {
            request,
            phase: ChartScreenshotCapturePhase::AwaitingBounds,
        }
    }

    pub(crate) fn request(&self) -> ChartScreenshotCaptureRequest {
        self.request
    }

    pub(crate) fn is_awaiting_bounds(&self, request: ChartScreenshotCaptureRequest) -> bool {
        self.request == request && self.phase == ChartScreenshotCapturePhase::AwaitingBounds
    }

    pub(crate) fn begin_rendering(&mut self, request: ChartScreenshotCaptureRequest) -> bool {
        if !self.is_awaiting_bounds(request) {
            return false;
        }
        self.phase = ChartScreenshotCapturePhase::Rendering;
        true
    }

    pub(crate) fn is_rendering(&self, request: ChartScreenshotCaptureRequest) -> bool {
        self.request == request && self.phase == ChartScreenshotCapturePhase::Rendering
    }
}

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
            .field("symbol", &format_args!("<redacted>"))
            .field("timeframe", &format_args!("<redacted>"))
            .field("width", &self.width)
            .field("height", &self.height)
            .field("rgba_len", &self.rgba.len())
            .field("png_len", &self.png.len())
            .field("captured_at", &format_args!("<redacted>"))
            .field("default_filename", &format_args!("<redacted>"))
            .finish()
    }
}

#[cfg(test)]
mod tests;

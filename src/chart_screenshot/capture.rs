use super::ChartScreenshotState;
use super::bitmap::encode_png_rgba;
use super::widget::ChartCanvasSnapshot;

use chrono::{DateTime, Local};
use iced::advanced::graphics::geometry::Renderer as GeometryRenderer;
use iced::advanced::renderer::Headless;
use iced::widget::image::Handle as ImageHandle;
use iced::{Color, Font, Pixels, Rectangle, Theme, mouse};
use std::sync::Arc;

mod io;
mod sizing;

pub(super) use io::{
    chart_screenshot_filename, copy_chart_screenshot_to_clipboard, save_chart_screenshot_png,
};
use sizing::ExportResolution;

// ---------------------------------------------------------------------------
// Capture Pipeline
// ---------------------------------------------------------------------------

pub(super) struct ChartScreenshotRenderRequest {
    pub(super) symbol: String,
    pub(super) timeframe: String,
    pub(super) chart: crate::chart::CandlestickChart,
    pub(super) background_color: Color,
    pub(super) captured_at: DateTime<Local>,
    pub(super) theme: Theme,
}

pub(super) async fn render_chart_screenshot(
    request: ChartScreenshotRenderRequest,
    snapshot: ChartCanvasSnapshot,
) -> Result<ChartScreenshotState, String> {
    let resolution = ExportResolution::new(snapshot.size)?;
    let mut renderer = <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), None)
        .await
        .ok_or_else(|| "offscreen chart renderer unavailable".to_string())?;

    render_with_renderer(request, snapshot, resolution, &mut renderer)
}

fn render_with_renderer(
    request: ChartScreenshotRenderRequest,
    snapshot: ChartCanvasSnapshot,
    resolution: ExportResolution,
    renderer: &mut iced::Renderer,
) -> Result<ChartScreenshotState, String> {
    let (width, height) = (resolution.size.width, resolution.size.height);
    let bounds = Rectangle::with_size(snapshot.size);
    let layers = request.chart.draw_with_state(
        &snapshot.state,
        renderer,
        &request.theme,
        bounds,
        mouse::Cursor::Unavailable,
    );
    for layer in layers {
        renderer.draw_geometry(layer);
    }

    let rgba = renderer.screenshot(
        resolution.size,
        resolution.scale_factor,
        request.background_color,
    );
    let png = encode_png_rgba(width, height, &rgba)?;
    let preview_handle = ImageHandle::from_rgba(width, height, rgba.clone());
    let captured_at = request.captured_at;
    let default_filename =
        chart_screenshot_filename(&request.symbol, &request.timeframe, captured_at);

    Ok(ChartScreenshotState {
        symbol: request.symbol,
        timeframe: request.timeframe,
        width,
        height,
        rgba: Arc::from(rgba),
        png: Arc::from(png),
        preview_handle,
        captured_at,
        default_filename,
    })
}

#[cfg(test)]
mod tests;

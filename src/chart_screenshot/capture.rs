use super::ChartScreenshotState;
use super::bitmap::encode_png_rgba;
use super::label::{ChartScreenshotLabelStyle, draw_ticker_label};

use chrono::Local;
use iced::advanced::graphics::geometry::Renderer as GeometryRenderer;
use iced::advanced::renderer::Headless;
use iced::widget::image::Handle as ImageHandle;
use iced::{Color, Font, Pixels, Rectangle, Size, Theme, mouse};
use std::sync::Arc;

mod io;
mod sizing;

pub(super) use io::{
    chart_screenshot_filename, copy_chart_screenshot_to_clipboard, save_chart_screenshot_png,
};
pub(super) use sizing::chart_screenshot_export_size;
#[cfg(test)]
pub(super) use sizing::{CHART_SCREENSHOT_MAX_EXPORT_EDGE, chart_screenshot_export_dimensions};

// ---------------------------------------------------------------------------
// Capture Pipeline
// ---------------------------------------------------------------------------

pub(super) struct ChartScreenshotRenderRequest {
    pub(super) symbol: String,
    pub(super) timeframe: String,
    pub(super) chart: crate::chart::CandlestickChart,
    pub(super) viewport: Option<crate::chart::ChartViewport>,
    pub(super) label_style: ChartScreenshotLabelStyle,
    pub(super) background_color: Color,
    pub(super) logical_bounds: Rectangle,
    pub(super) theme: Theme,
}

pub(super) async fn render_chart_screenshot(
    request: ChartScreenshotRenderRequest,
) -> Result<ChartScreenshotState, String> {
    let (width, height) = chart_screenshot_export_size(request.logical_bounds)?;
    let mut renderer = <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), None)
        .await
        .ok_or_else(|| "offscreen chart renderer unavailable".to_string())?;

    let bounds = Rectangle {
        x: 0.0,
        y: 0.0,
        width: width as f32,
        height: height as f32,
    };
    let chart_w = (bounds.width - request.chart.price_axis_width()).max(1.0);
    let state =
        crate::chart::ChartState::for_export_viewport(&request.chart, request.viewport, chart_w);

    let layers = request.chart.draw_with_state(
        &state,
        &renderer,
        &request.theme,
        bounds,
        mouse::Cursor::Unavailable,
    );
    for layer in layers {
        renderer.draw_geometry(layer);
    }

    let mut rgba = renderer.screenshot(Size::new(width, height), 1.0, request.background_color);
    draw_ticker_label(
        &mut rgba,
        width,
        height,
        &request.symbol,
        &request.timeframe,
        request.label_style,
    );
    let png = encode_png_rgba(width, height, &rgba)?;
    let preview_handle = ImageHandle::from_rgba(width, height, rgba.clone());
    let captured_at = Local::now();
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

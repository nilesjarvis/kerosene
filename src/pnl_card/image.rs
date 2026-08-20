use crate::chart_screenshot::encode_png_rgba;
use crate::denomination::DisplayDenominationContext;

use super::display_text::pnl_card_render_text;
use super::metrics::PnlCardMetrics;
use super::model::PnlCardWindowState;
use super::rendering::PnlCardCanvas;

use iced::advanced::graphics::geometry::Renderer as GeometryRenderer;
use iced::advanced::renderer::Headless;
use iced::{Color, Pixels, Rectangle, Size, Theme};

mod formatting;
mod io;

pub(super) use formatting::pnl_card_filename;
pub(super) use io::{copy_pnl_card_to_clipboard, save_pnl_card_png};

// ---------------------------------------------------------------------------
// Image Export
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(super) struct PnlCardImage {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) rgba: Vec<u8>,
    pub(super) png: Vec<u8>,
    pub(super) default_filename: String,
}

#[derive(Debug, Clone)]
pub(super) struct PnlCardRenderRequest {
    state: PnlCardWindowState,
    metrics: PnlCardMetrics,
    denomination: DisplayDenominationContext,
    pnl_color: Color,
    theme: Theme,
}

impl PnlCardRenderRequest {
    pub(super) fn new(
        state: PnlCardWindowState,
        metrics: PnlCardMetrics,
        denomination: DisplayDenominationContext,
        pnl_color: Color,
        theme: Theme,
    ) -> Self {
        Self {
            state,
            metrics,
            denomination,
            pnl_color,
            theme,
        }
    }
}

pub(super) async fn render_pnl_card_image(
    request: PnlCardRenderRequest,
) -> Result<PnlCardImage, String> {
    const WIDTH: u32 = 1200;
    const HEIGHT: u32 = 675;

    let render_text = pnl_card_render_text(&request.state, &request.metrics, &request.denomination);
    let card = PnlCardCanvas::new(render_text, request.metrics.upnl, request.pnl_color);
    let mut renderer = <iced::Renderer as Headless>::new(card.font(), Pixels(16.0), None)
        .await
        .ok_or_else(|| "offscreen P&L card renderer unavailable".to_string())?;
    let bounds = Rectangle::with_size(Size::new(WIDTH as f32, HEIGHT as f32));
    for layer in card.draw(&renderer, &request.theme, bounds) {
        renderer.draw_geometry(layer);
    }

    let rgba = renderer.screenshot(
        Size::new(WIDTH, HEIGHT),
        1.0,
        request.theme.palette().background,
    );

    let png = encode_png_rgba(WIDTH, HEIGHT, &rgba)?;
    let default_filename = pnl_card_filename(card.ticker());

    Ok(PnlCardImage {
        width: WIDTH,
        height: HEIGHT,
        rgba,
        png,
        default_filename,
    })
}

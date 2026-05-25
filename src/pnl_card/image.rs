use crate::chart_screenshot::{
    PixelPoint, bitmap_text_width, color_to_rgba, draw_bitmap_text, encode_png_rgba,
};
use crate::denomination::DisplayDenominationContext;

use super::display_text::pnl_card_render_text;
use super::metrics::PnlCardMetrics;
use super::model::PnlCardWindowState;
use super::style::pnl_card_palette;

use iced::{Color, Theme};

mod drawing;
mod formatting;
mod io;

use drawing::{
    ExportMetricStyle, best_text_scale, draw_export_metric, draw_pnl_card_export_border,
    draw_pnl_card_gradient,
};
pub(super) use formatting::{export_text, pnl_card_filename};
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

pub(super) fn render_pnl_card_image(
    state: &PnlCardWindowState,
    metrics: PnlCardMetrics,
    denomination: DisplayDenominationContext,
    pnl_color: Color,
    theme: &Theme,
) -> Result<PnlCardImage, String> {
    const WIDTH: u32 = 1200;
    const HEIGHT: u32 = 675;

    let mut rgba = vec![0; WIDTH as usize * HEIGHT as usize * 4];
    draw_pnl_card_gradient(&mut rgba, WIDTH, HEIGHT, pnl_color, theme);

    let card_palette = pnl_card_palette(theme, pnl_color);
    let text_rgba = color_to_rgba(card_palette.text, 255);
    let weak_rgba = color_to_rgba(card_palette.weak_text, 232);
    let render_text = pnl_card_render_text(state, &metrics, &denomination);
    let primary_value = export_text(&render_text.primary_value);
    let secondary_value = render_text
        .secondary_value
        .as_ref()
        .map(|value| export_text(value));
    let entry_display = export_text(&render_text.entry_display);
    let exit_display = export_text(&render_text.exit_display);
    let ticker = export_text(&render_text.ticker);
    let context = export_text(&render_text.context);
    let leverage_display = export_text(&render_text.leverage_display);

    draw_pnl_card_export_border(&mut rgba, WIDTH, HEIGHT, pnl_color, theme);

    draw_bitmap_text(
        &mut rgba,
        WIDTH,
        HEIGHT,
        PixelPoint { x: 60, y: 54 },
        5,
        "KEROSENE",
        text_rgba,
    );

    let ticker_scale = best_text_scale(&ticker, 430, 8, 3);
    let ticker_width = bitmap_text_width(&ticker, ticker_scale);
    draw_bitmap_text(
        &mut rgba,
        WIDTH,
        HEIGHT,
        PixelPoint {
            x: WIDTH.saturating_sub(60 + ticker_width),
            y: 48,
        },
        ticker_scale,
        &ticker,
        text_rgba,
    );

    let primary_scale = best_text_scale(&primary_value, WIDTH - 120, 15, 5);
    draw_bitmap_text(
        &mut rgba,
        WIDTH,
        HEIGHT,
        PixelPoint { x: 60, y: 226 },
        primary_scale,
        &primary_value,
        text_rgba,
    );

    let percent_mode = export_text(state.percent_mode.label());
    draw_bitmap_text(
        &mut rgba,
        WIDTH,
        HEIGHT,
        PixelPoint { x: 64, y: 356 },
        4,
        &percent_mode,
        weak_rgba,
    );

    if let Some(secondary_value) = secondary_value {
        draw_bitmap_text(
            &mut rgba,
            WIDTH,
            HEIGHT,
            PixelPoint { x: 60, y: 398 },
            best_text_scale(&secondary_value, WIDTH - 120, 7, 3),
            &secondary_value,
            text_rgba,
        );
    }

    let metric_style = ExportMetricStyle {
        width: WIDTH,
        height: HEIGHT,
        label_color: weak_rgba,
        value_color: text_rgba,
    };
    draw_export_metric(
        &mut rgba,
        metric_style,
        PixelPoint { x: 60, y: 506 },
        "LEV",
        &leverage_display,
    );
    draw_export_metric(
        &mut rgba,
        metric_style,
        PixelPoint { x: 420, y: 506 },
        "ENTRY",
        &entry_display,
    );
    draw_export_metric(
        &mut rgba,
        metric_style,
        PixelPoint { x: 780, y: 506 },
        "EXIT",
        &exit_display,
    );

    let context_scale = best_text_scale(&context, WIDTH - 120, 3, 2);
    draw_bitmap_text(
        &mut rgba,
        WIDTH,
        HEIGHT,
        PixelPoint {
            x: 60,
            y: if context_scale <= 2 { 590 } else { 586 },
        },
        context_scale,
        &context,
        weak_rgba,
    );

    let png = encode_png_rgba(WIDTH, HEIGHT, &rgba)?;
    let default_filename = pnl_card_filename(&metrics.ticker);

    Ok(PnlCardImage {
        width: WIDTH,
        height: HEIGHT,
        rgba,
        png,
        default_filename,
    })
}

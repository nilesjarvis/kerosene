use super::bitmap::{
    BITMAP_GLYPH_HEIGHT, PixelPoint, Rect, bitmap_text_width, color_to_rgba, draw_bitmap_text,
    fill_rect, stroke_rect,
};

use iced::Theme;

mod text;

pub(crate) use text::ticker_label_text;

// ---------------------------------------------------------------------------
// Screenshot Label
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChartScreenshotLabelStyle {
    pub(super) background: [u8; 4],
    pub(super) border: [u8; 4],
    pub(super) accent: [u8; 4],
    pub(super) text: [u8; 4],
}

pub(crate) fn chart_screenshot_label_style(theme: &Theme) -> ChartScreenshotLabelStyle {
    let palette = theme.palette();
    let extended = theme.extended_palette();

    ChartScreenshotLabelStyle {
        background: color_to_rgba(extended.background.weak.color, 230),
        border: color_to_rgba(extended.background.strong.color, 145),
        accent: color_to_rgba(palette.primary, 210),
        text: color_to_rgba(palette.text, 248),
    }
}

pub(crate) fn draw_ticker_label(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    symbol: &str,
    timeframe: &str,
    style: ChartScreenshotLabelStyle,
) {
    if width < 72 || height < 28 || rgba.len() != width as usize * height as usize * 4 {
        return;
    }

    let scale = if width < 360 || height < 220 { 1 } else { 2 };
    let padding_x = 6 * scale;
    let padding_y = 5 * scale;
    let x = 8 * scale;
    let y = 8 * scale;
    let available_width = width.saturating_sub(x + padding_x * 2 + 4);
    let text = ticker_label_text(symbol, timeframe, available_width, scale);
    if text.is_empty() {
        return;
    }

    let text_w = bitmap_text_width(&text, scale);
    let text_h = BITMAP_GLYPH_HEIGHT * scale;
    let accent_w = 2 * scale;
    let accent_gap = 3 * scale;
    let label_w = text_w + padding_x * 2 + accent_w + accent_gap;
    let label_h = text_h + padding_y * 2;
    if x + label_w >= width || y + label_h >= height {
        return;
    }

    fill_rect(
        rgba,
        width,
        height,
        Rect {
            x,
            y,
            width: label_w,
            height: label_h,
        },
        style.background,
    );
    stroke_rect(
        rgba,
        width,
        height,
        Rect {
            x,
            y,
            width: label_w,
            height: label_h,
        },
        style.border,
    );
    fill_rect(
        rgba,
        width,
        height,
        Rect {
            x: x + 1,
            y: y + 1,
            width: accent_w,
            height: label_h.saturating_sub(2),
        },
        style.accent,
    );
    draw_bitmap_text(
        rgba,
        width,
        height,
        PixelPoint {
            x: x + padding_x + accent_w + accent_gap,
            y: y + padding_y,
        },
        scale,
        &text,
        style.text,
    );
}

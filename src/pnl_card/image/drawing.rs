use crate::chart_screenshot::{PixelPoint, Rect, bitmap_text_width, draw_bitmap_text, fill_rect};

use super::super::style::{detail_band_rgba, mix_color, pnl_card_palette};

use iced::{Color, Theme};
use std::fmt;

// ---------------------------------------------------------------------------
// Bitmap Drawing Helpers
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub(super) struct ExportMetricStyle {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) label_color: [u8; 4],
    pub(super) value_color: [u8; 4],
}

impl fmt::Debug for ExportMetricStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExportMetricStyle")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("label_color", &format_args!("<redacted>"))
            .field("value_color", &format_args!("<redacted>"))
            .finish()
    }
}

pub(super) fn draw_export_metric(
    rgba: &mut [u8],
    style: ExportMetricStyle,
    origin: PixelPoint,
    label: &'static str,
    value: &str,
) {
    draw_bitmap_text(
        rgba,
        style.width,
        style.height,
        origin,
        3,
        label,
        style.label_color,
    );
    draw_bitmap_text(
        rgba,
        style.width,
        style.height,
        PixelPoint {
            x: origin.x,
            y: origin.y + 34,
        },
        best_text_scale(value, 320, 5, 2),
        value,
        style.value_color,
    );
}

pub(super) fn draw_pnl_card_gradient(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    pnl_color: Color,
    theme: &Theme,
) {
    let card_palette = pnl_card_palette(theme, pnl_color);
    let shadow = mix_color(card_palette.end, Color::BLACK, 0.20);

    for y in 0..height {
        for x in 0..width {
            let t =
                (x as f32 * 0.72 + y as f32 * 0.28) / (width as f32 * 0.72 + height as f32 * 0.28);
            let color = if t < 0.58 {
                mix_color(card_palette.start, card_palette.mid, t / 0.58)
            } else {
                mix_color(card_palette.mid, shadow, (t - 0.58) / 0.42)
            };
            let idx = (y as usize * width as usize + x as usize) * 4;
            rgba[idx] = color_to_byte(color.r);
            rgba[idx + 1] = color_to_byte(color.g);
            rgba[idx + 2] = color_to_byte(color.b);
            rgba[idx + 3] = 255;
        }
    }

    fill_rect(
        rgba,
        width,
        height,
        Rect {
            x: 0,
            y: height.saturating_sub(184),
            width,
            height: 184,
        },
        detail_band_rgba(pnl_card_palette(theme, pnl_color).text, 44),
    );
}

pub(super) fn draw_pnl_card_export_border(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    pnl_color: Color,
    theme: &Theme,
) {
    let palette = pnl_card_palette(theme, pnl_color);
    let border_width = 22;
    for y in 24..height.saturating_sub(24) {
        for x in 24..width.saturating_sub(24) {
            let in_left = x < 24 + border_width;
            let in_right = x >= width.saturating_sub(24 + border_width);
            let in_top = y < 24 + border_width;
            let in_bottom = y >= height.saturating_sub(24 + border_width);
            if !(in_left || in_right || in_top || in_bottom) {
                continue;
            }

            let t = (x as f32 + y as f32) / (width as f32 + height as f32);
            let color = if t < 0.5 {
                mix_color(palette.border_start, palette.border_mid, t * 2.0)
            } else {
                mix_color(palette.border_mid, palette.border_end, (t - 0.5) * 2.0)
            };
            set_pixel(rgba, width, x, y, color);
        }
    }
}

fn set_pixel(rgba: &mut [u8], width: u32, x: u32, y: u32, color: Color) {
    let idx = (y as usize * width as usize + x as usize) * 4;
    if idx + 3 >= rgba.len() {
        return;
    }

    rgba[idx] = color_to_byte(color.r);
    rgba[idx + 1] = color_to_byte(color.g);
    rgba[idx + 2] = color_to_byte(color.b);
    rgba[idx + 3] = 255;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_metric_style_debug_redacts_derived_colors() {
        let style = ExportMetricStyle {
            width: 1200,
            height: 675,
            label_color: [241, 242, 243, 244],
            value_color: [231, 232, 233, 234],
        };

        let rendered = format!("{style:?}");

        assert!(rendered.contains("width: 1200"), "{rendered}");
        assert!(rendered.contains("height: 675"), "{rendered}");
        assert!(rendered.contains("<redacted>"), "{rendered}");
        assert!(!rendered.contains("241, 242, 243, 244"), "{rendered}");
        assert!(!rendered.contains("231, 232, 233, 234"), "{rendered}");
    }
}

fn color_to_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub(super) fn best_text_scale(text: &str, max_width: u32, preferred: u32, minimum: u32) -> u32 {
    (minimum..=preferred)
        .rev()
        .find(|scale| bitmap_text_width(text, *scale) <= max_width)
        .unwrap_or(minimum)
}

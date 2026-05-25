mod glyphs;
mod primitives;
pub(crate) use glyphs::{
    BITMAP_GLYPH_HEIGHT, bitmap_max_chars, bitmap_text_width, is_bitmap_glyph_supported,
};
pub(crate) use primitives::{
    PixelPoint, Rect, color_to_rgba, encode_png_rgba, fill_rect, stroke_rect,
};

use glyphs::{BITMAP_GLYPH_WIDTH, bitmap_glyph};

// ---------------------------------------------------------------------------
// Bitmap Utilities
// ---------------------------------------------------------------------------

pub(crate) fn draw_bitmap_text(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    origin: PixelPoint,
    scale: u32,
    text: &str,
    color: [u8; 4],
) {
    let mut cursor_x = origin.x;
    for ch in text.chars() {
        let glyph = bitmap_glyph(ch);
        for (row_idx, row) in glyph.iter().enumerate() {
            for col in 0..BITMAP_GLYPH_WIDTH {
                if *row & (1 << (BITMAP_GLYPH_WIDTH - 1 - col)) == 0 {
                    continue;
                }
                let px = cursor_x + col * scale;
                let py = origin.y + row_idx as u32 * scale;
                fill_rect(
                    rgba,
                    width,
                    height,
                    Rect {
                        x: px,
                        y: py,
                        width: scale,
                        height: scale,
                    },
                    color,
                );
            }
        }
        cursor_x += (BITMAP_GLYPH_WIDTH + 1) * scale;
    }
}

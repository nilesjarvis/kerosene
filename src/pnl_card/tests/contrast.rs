use super::*;

#[test]
fn card_palette_keeps_text_readable_across_builtin_themes() {
    let pnl_colors = [
        Color::from_rgb8(0x50, 0xfa, 0x7b),
        Color::from_rgb8(0xff, 0x55, 0x55),
    ];

    for theme in Theme::ALL {
        for pnl_color in pnl_colors {
            let palette = pnl_card_palette(theme, pnl_color);
            let min_contrast =
                minimum_contrast_ratio(palette.text, &[palette.start, palette.mid, palette.end]);

            assert!(
                min_contrast >= 4.5,
                "theme {theme:?} contrast {min_contrast:.2} is too low"
            );
        }
    }
}

use iced::Color;

// ---------------------------------------------------------------------------
// UI Colors
// ---------------------------------------------------------------------------

pub fn text_color_for_bg(bg: Color) -> Color {
    let lum =
        bg.into_linear()[0] * 0.2126 + bg.into_linear()[1] * 0.7152 + bg.into_linear()[2] * 0.0722;
    if lum > 0.4 {
        Color::from_rgb(0.05, 0.05, 0.05)
    } else {
        Color::from_rgb(0.95, 0.95, 0.95)
    }
}

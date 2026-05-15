use iced::Color;

// ---------------------------------------------------------------------------
// UI Colors
// ---------------------------------------------------------------------------

pub fn text_color_for_bg(bg: Color) -> Color {
    let [red, green, blue, _alpha] = bg.into_linear();
    let lum = red * 0.2126 + green * 0.7152 + blue * 0.0722;
    if lum > 0.4 {
        Color::from_rgb(0.05, 0.05, 0.05)
    } else {
        Color::from_rgb(0.95, 0.95, 0.95)
    }
}

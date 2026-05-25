use iced::{Color, Theme};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// UI Colors
// ---------------------------------------------------------------------------

pub fn optional_value_color<T>(
    value: Option<T>,
    default_color: Color,
    invalid_color: Color,
) -> Color {
    if value.is_some() {
        default_color
    } else {
        invalid_color
    }
}

pub fn signed_number_color(value: f64, theme: &Theme) -> Color {
    if value > 0.0 {
        theme.palette().success
    } else if value < 0.0 {
        theme.palette().danger
    } else {
        theme.palette().text
    }
}

pub fn text_color_for_bg(bg: Color) -> Color {
    let [red, green, blue, _alpha] = bg.into_linear();
    let lum = red * 0.2126 + green * 0.7152 + blue * 0.0722;
    if lum > 0.4 {
        Color::from_rgb(0.05, 0.05, 0.05)
    } else {
        Color::from_rgb(0.95, 0.95, 0.95)
    }
}

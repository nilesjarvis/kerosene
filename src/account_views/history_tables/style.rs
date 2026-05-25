use iced::{Color, Theme};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// History Table Styling
// ---------------------------------------------------------------------------

pub(super) fn history_signed_value_color(value: Option<f64>, theme: &Theme) -> Color {
    match value {
        Some(value) if value >= 0.0 => theme.palette().success,
        Some(_) => theme.palette().danger,
        None => theme.palette().warning,
    }
}

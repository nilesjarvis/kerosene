use crate::app_state::TradingTerminal;
use iced::{Color, Theme};

mod probability;
mod side_button;

// ---------------------------------------------------------------------------
// Outcome Components
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn outcome_side_accent(theme: &Theme, side_name: &str, side_index: u32) -> Color {
        match side_name.trim().to_ascii_lowercase().as_str() {
            "yes" => theme.palette().success,
            "no" => theme.palette().danger,
            _ => match side_index % 5 {
                0 => theme.palette().primary,
                1 => theme.extended_palette().secondary.base.color,
                2 => theme.palette().warning,
                3 => theme.palette().success,
                _ => theme.extended_palette().primary.strong.color,
            },
        }
    }
}

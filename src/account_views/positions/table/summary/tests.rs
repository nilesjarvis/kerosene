use super::super::format_position_usd_value;
use super::*;

mod account_values;
mod formatting;
mod totals;

fn format_unsigned_usd(value: f64, hide_pnl: bool, number_mode: PositionNumberMode) -> String {
    if hide_pnl {
        "$***".to_string()
    } else {
        format_position_usd_value(value, number_mode)
    }
}

fn format_optional_unsigned_usd(
    total: Option<f64>,
    hide_pnl: bool,
    number_mode: PositionNumberMode,
) -> String {
    match total {
        Some(value) => format_unsigned_usd(value, hide_pnl, number_mode),
        None => "--".to_string(),
    }
}

fn format_optional_signed_usd(
    total: Option<f64>,
    hide_pnl: bool,
    number_mode: PositionNumberMode,
) -> String {
    match total {
        Some(_) if hide_pnl => "$***".to_string(),
        Some(value) => format_signed_usd(value, number_mode),
        None => "--".to_string(),
    }
}

fn format_optional_total_pnl(
    total: Option<f64>,
    percent: Option<f64>,
    hide_pnl: bool,
    number_mode: PositionNumberMode,
) -> String {
    match total {
        Some(_) if hide_pnl => {
            let percent = percent
                .map(|percent| format_signed_percent(percent, number_mode))
                .unwrap_or_else(|| "--%".to_string());
            format!("$*** ({percent})")
        }
        Some(value) => {
            let percent = percent
                .map(|percent| format_signed_percent(percent, number_mode))
                .unwrap_or_else(|| "--%".to_string());
            format!("{} ({percent})", format_signed_usd(value, number_mode))
        }
        None => "--".to_string(),
    }
}

fn format_signed_usd(value: f64, number_mode: PositionNumberMode) -> String {
    let min_display = if number_mode.is_compact() { 0.5 } else { 0.005 };
    let display_value = if value.abs() < min_display {
        0.0
    } else {
        value
    };
    let formatted = format_position_usd_value(display_value, number_mode);
    if display_value > 0.0 {
        format!("+{formatted}")
    } else {
        formatted
    }
}

use super::super::super::PositionNumberMode;
use super::super::format_position_display_value;
use super::totals::OptionalTotal;
use crate::denomination::DisplayDenominationContext;

// ---------------------------------------------------------------------------
// Summary Formatting
// ---------------------------------------------------------------------------

pub(super) fn format_unsigned_display(
    context: &DisplayDenominationContext,
    value: f64,
    hide_pnl: bool,
    number_mode: PositionNumberMode,
) -> String {
    if hide_pnl {
        context.hidden_mask()
    } else {
        format_position_display_value(context, value, number_mode)
    }
}

pub(super) fn format_optional_unsigned_display(
    context: &DisplayDenominationContext,
    total: OptionalTotal,
    hide_pnl: bool,
    number_mode: PositionNumberMode,
) -> String {
    match total.value() {
        Some(value) => format_unsigned_display(context, value, hide_pnl, number_mode),
        None => "--".to_string(),
    }
}

pub(super) fn format_optional_signed_display(
    context: &DisplayDenominationContext,
    total: OptionalTotal,
    hide_pnl: bool,
    number_mode: PositionNumberMode,
) -> String {
    match total.value() {
        Some(_) if hide_pnl => context.hidden_mask(),
        Some(value) => format_signed_display(context, value, number_mode),
        None => "--".to_string(),
    }
}

pub(super) fn format_optional_total_pnl_display(
    context: &DisplayDenominationContext,
    total: OptionalTotal,
    percent: Option<f64>,
    hide_pnl: bool,
    number_mode: PositionNumberMode,
) -> String {
    match total.value() {
        Some(_) if hide_pnl => {
            let percent = percent
                .map(|percent| format_signed_percent(percent, number_mode))
                .unwrap_or_else(|| "--%".to_string());
            format!("{} ({percent})", context.hidden_mask())
        }
        Some(value) => {
            let percent = percent
                .map(|percent| format_signed_percent(percent, number_mode))
                .unwrap_or_else(|| "--%".to_string());
            format!(
                "{} ({percent})",
                format_signed_display(context, value, number_mode)
            )
        }
        None => "--".to_string(),
    }
}

pub(super) fn format_signed_display(
    context: &DisplayDenominationContext,
    value: f64,
    number_mode: PositionNumberMode,
) -> String {
    let min_display = if number_mode.is_compact() { 0.5 } else { 0.005 };
    let display_value = if value.abs() < min_display {
        0.0
    } else {
        value
    };
    let formatted = format_position_display_value(context, display_value, number_mode);
    if display_value > 0.0 {
        format!("+{formatted}")
    } else {
        formatted
    }
}

pub(super) fn format_signed_percent(value: f64, number_mode: PositionNumberMode) -> String {
    let decimals = if number_mode.is_compact() { 1 } else { 2 };
    let min_display = if number_mode.is_compact() {
        0.05
    } else {
        0.005
    };
    let display_value = if value.abs() < min_display {
        0.0
    } else {
        value
    };
    if display_value > 0.0 {
        format!("+{display_value:.decimals$}%")
    } else {
        format!("{display_value:.decimals$}%")
    }
}

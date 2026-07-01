use crate::account_views::invalid_account_data;
pub(super) use crate::helpers::trim_decimal_zeros;
use crate::helpers::{format_decimal_with_commas, format_price};

use super::super::{PositionNumberMode, format_position_compact_number};

// ---------------------------------------------------------------------------
// Position Row Formatting
// ---------------------------------------------------------------------------

pub(super) fn format_position_signed_amount(
    context: &crate::denomination::DisplayDenominationContext,
    value: f64,
    number_mode: PositionNumberMode,
) -> String {
    match number_mode {
        PositionNumberMode::Full => context.format_signed_value(value, 2),
        PositionNumberMode::Compact => format_signed_compact_amount(context, value),
    }
}

fn format_signed_compact_amount(
    context: &crate::denomination::DisplayDenominationContext,
    value: f64,
) -> String {
    let Some(display_value) = context.convert_usd_value(value) else {
        return invalid_account_data();
    };
    let compact_value = format_position_compact_number(display_value.abs());
    let sign = if display_value > 0.0 && compact_value != "0" {
        "+"
    } else if display_value < 0.0 && compact_value != "0" {
        "-"
    } else {
        ""
    };
    context.format_active_amount(sign, compact_value)
}

pub(super) fn format_position_entry_price(entry_px: Option<f64>, raw: &str) -> String {
    let Some(entry_px) = entry_px else {
        return "Invalid".to_string();
    };
    if entry_px.abs() < 1_000.0 {
        return raw.to_string();
    }

    format_large_wire_price(raw).unwrap_or_else(|| format_price(entry_px))
}

pub(super) fn format_spot_position_entry_price(entry_px: Option<f64>) -> String {
    entry_px
        .map(|entry_px| format_decimal_with_commas(entry_px, 2))
        .unwrap_or_else(|| "Invalid".to_string())
}

fn format_large_wire_price(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (sign, unsigned) = trimmed
        .strip_prefix('-')
        .map(|value| ("-", value))
        .or_else(|| trimmed.strip_prefix('+').map(|value| ("+", value)))
        .unwrap_or(("", trimmed));
    let (whole, fraction) = unsigned
        .split_once('.')
        .map_or((unsigned, None), |(whole, fraction)| {
            (whole, Some(fraction))
        });
    if whole.is_empty() || !whole.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    if let Some(fraction) = fraction
        && !fraction.chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }

    let mut grouped = String::with_capacity(whole.len() + whole.len() / 3);
    for (i, ch) in whole.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let whole_grouped: String = grouped.chars().rev().collect();

    Some(match fraction {
        Some(fraction) => format!("{sign}{whole_grouped}.{fraction}"),
        None => format!("{sign}{whole_grouped}"),
    })
}

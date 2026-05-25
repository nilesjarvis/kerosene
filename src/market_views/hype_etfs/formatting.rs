use crate::denomination::DisplayDenominationContext;
use crate::helpers::{
    finite_value, format_decimal_with_commas, format_size, normalize_two_decimal_display_value,
    not_available_placeholder,
};

// ---------------------------------------------------------------------------
// HYPE ETF Formatting
// ---------------------------------------------------------------------------

pub(super) fn format_usd_value(
    value: Option<f64>,
    decimals: usize,
    denomination: &DisplayDenominationContext,
) -> String {
    format_finite_value(value, |value| denomination.format_value(value, decimals))
}

pub(super) fn format_amount(value: Option<f64>) -> String {
    format_finite_value(value, format_size)
}

pub(super) fn format_hype(value: Option<f64>) -> String {
    format_finite_value(value, |value| {
        format!("{} HYPE", format_decimal_with_commas(value, 0))
    })
}

pub(super) fn format_pct(value: Option<f64>) -> String {
    format_finite_value(value, |value| format!("{value:+.2}%"))
}

pub(super) fn format_signed_usd_amount(
    value: f64,
    denomination: &DisplayDenominationContext,
) -> String {
    denomination.format_signed_value(normalize_two_decimal_display_value(value), 2)
}

pub(super) fn short_flow_date(date: &str) -> String {
    date.get(5..).unwrap_or(date).replace('-', "/")
}

fn format_finite_value(value: Option<f64>, format: impl FnOnce(f64) -> String) -> String {
    value
        .and_then(finite_value)
        .map(format)
        .unwrap_or_else(not_available_placeholder)
}

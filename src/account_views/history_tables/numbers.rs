use crate::account_views::invalid_account_data;
#[cfg(test)]
use crate::helpers::format_usd;
use crate::helpers::parse_finite_number;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// History Table Number Formatting
// ---------------------------------------------------------------------------

pub(super) fn parse_history_number(value: &str) -> Option<f64> {
    parse_finite_number(value)
}

pub(super) fn valid_history_wire_value(value: &str) -> String {
    parse_history_number(value)
        .map(|_| value.to_string())
        .unwrap_or_else(invalid_history_data)
}

#[cfg(test)]
pub(super) fn format_history_usd(value: Option<f64>, decimals: usize) -> String {
    value
        .map(|value| format_usd(&format!("{value:.decimals$}")))
        .unwrap_or_else(invalid_history_data)
}

pub(super) fn format_history_display_usd(
    context: &crate::denomination::DisplayDenominationContext,
    value: Option<f64>,
    decimals: usize,
) -> String {
    value
        .map(|value| context.format_value(value, decimals))
        .unwrap_or_else(invalid_history_data)
}

pub(super) fn invalid_history_data() -> String {
    invalid_account_data()
}

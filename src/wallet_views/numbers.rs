use crate::helpers::format_price;
#[cfg(test)]
use crate::helpers::format_usd;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Wallet Detail Number Formatting
// ---------------------------------------------------------------------------

pub(super) fn parse_wallet_number(value: &str) -> Option<f64> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

pub(super) fn wallet_has_visible_nonzero(value: &str) -> bool {
    parse_wallet_number(value)
        .map(|value| value.abs() > f64::EPSILON)
        .unwrap_or(true)
}

#[cfg(test)]
pub(super) fn format_wallet_usd(value: Option<f64>, decimals: usize) -> String {
    value
        .map(|value| format_usd(&format!("{value:.decimals$}")))
        .unwrap_or_else(invalid_wallet_data)
}

pub(super) fn format_wallet_display_usd(
    denomination: &crate::denomination::DisplayDenominationContext,
    value: Option<f64>,
    decimals: usize,
) -> String {
    value
        .map(|value| denomination.format_value(value, decimals))
        .unwrap_or_else(invalid_wallet_data)
}

pub(super) fn format_wallet_display_signed_usd(
    denomination: &crate::denomination::DisplayDenominationContext,
    value: Option<f64>,
) -> String {
    value
        .map(|value| denomination.format_signed_value(value, 2))
        .unwrap_or_else(invalid_wallet_data)
}

pub(super) fn format_wallet_price(value: Option<f64>) -> String {
    value.map(format_price).unwrap_or_else(invalid_wallet_data)
}

#[cfg(test)]
pub(super) fn format_wallet_amount(value: Option<f64>, is_usdc: bool) -> String {
    match value {
        Some(value) if is_usdc => format_wallet_usd(Some(value), 2),
        Some(value) => format!("{value:.6}"),
        None => invalid_wallet_data(),
    }
}

pub(super) fn format_wallet_display_amount(
    denomination: &crate::denomination::DisplayDenominationContext,
    value: Option<f64>,
    is_usdc: bool,
) -> String {
    match value {
        Some(value) if is_usdc => denomination.format_value(value, 2),
        Some(value) => format!("{value:.6}"),
        None => invalid_wallet_data(),
    }
}

pub(super) fn invalid_wallet_data() -> String {
    "Invalid data".to_string()
}

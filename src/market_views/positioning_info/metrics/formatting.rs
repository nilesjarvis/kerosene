use crate::denomination::DisplayDenominationContext;
use crate::helpers;
use crate::wallet_state::address_book::WalletDisplay;

use iced::{Color, Theme};

// ---------------------------------------------------------------------------
// Positioning Presentation Helpers
// ---------------------------------------------------------------------------

pub(in crate::market_views::positioning_info) fn signed_value_color(
    value: f64,
    theme: &Theme,
) -> Color {
    helpers::signed_number_color(value, theme)
}

pub(in crate::market_views::positioning_info) fn position_identity(
    wallet_display: WalletDisplay,
) -> String {
    wallet_display.primary
}

pub(in crate::market_views::positioning_info) fn trader_text_limit(
    width: f32,
    max_chars: usize,
) -> usize {
    let estimated_chars = ((width.max(0.0) - 8.0).max(0.0) / 6.4).floor() as usize;
    estimated_chars.clamp(8, max_chars)
}

pub(in crate::market_views::positioning_info) fn truncate_ascii(
    value: &str,
    max_chars: usize,
) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut truncated: String = value.chars().take(max_chars.saturating_sub(3)).collect();
    truncated.push_str("...");
    truncated
}

pub(in crate::market_views::positioning_info) fn position_side_label(size: f64) -> &'static str {
    if size > 0.0 {
        "\u{2191} Long"
    } else if size < 0.0 {
        "\u{2193} Short"
    } else {
        "Flat"
    }
}

pub(in crate::market_views::positioning_info) fn position_side_color(
    size: f64,
    theme: &Theme,
) -> Color {
    if size > 0.0 {
        theme.palette().success
    } else if size < 0.0 {
        theme.palette().danger
    } else {
        theme.extended_palette().background.weak.text
    }
}

pub(in crate::market_views::positioning_info) fn format_usd_number(
    value: f64,
    denomination: &DisplayDenominationContext,
) -> String {
    if value.is_finite() {
        denomination.format_value(value, 2)
    } else {
        "-".to_string()
    }
}

pub(in crate::market_views::positioning_info) fn format_signed_usd(
    value: f64,
    denomination: &DisplayDenominationContext,
) -> String {
    if value.is_finite() {
        denomination.format_signed_value(value, 2)
    } else {
        "-".to_string()
    }
}

pub(in crate::market_views::positioning_info) fn format_price_number(
    value: f64,
    denomination: &DisplayDenominationContext,
) -> String {
    if helpers::positive_finite_value(value).is_some() {
        denomination.format_price(value)
    } else {
        "-".to_string()
    }
}

pub(in crate::market_views::positioning_info) fn format_signed_size(
    value: f64,
    plus_for_positive: bool,
) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }
    let size = helpers::format_size(value.abs());
    if value > 0.0 && plus_for_positive {
        format!("+{size}")
    } else if value < 0.0 {
        format!("-{size}")
    } else {
        size
    }
}

pub(in crate::market_views::positioning_info) fn format_positioning_timestamp(
    timestamp: &str,
) -> String {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| {
            dt.with_timezone(&chrono::Local)
                .format("%b %d, %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| timestamp.to_string())
}

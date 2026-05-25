use crate::denomination::DisplayDenominationContext;
use crate::feed_state::TrackedTradeIntent;
use crate::helpers::format_decimal_with_commas;

pub(super) fn tracked_trade_side_label(is_buy: bool) -> &'static str {
    if is_buy { "BUY" } else { "SELL" }
}

pub(super) fn tracked_trade_fee_label(fee: f64, fee_token: &str) -> String {
    let fee = format_trimmed_number(fee, 4);
    if fee_token.trim().is_empty() {
        fee
    } else {
        format!("{fee} {fee_token}")
    }
}

pub(super) fn tracked_trade_size_label(size: f64) -> String {
    format_trimmed_number(size, 4)
}

pub(super) fn tracked_trade_price_label(price: f64) -> String {
    format_trimmed_number(price, 4)
}

pub(super) fn tracked_trade_notional_label(
    denomination: &DisplayDenominationContext,
    notional: f64,
) -> String {
    format_display_trimmed(denomination, notional, false)
}

pub(super) fn tracked_trade_pnl_label(
    denomination: &DisplayDenominationContext,
    pnl: f64,
) -> String {
    format_display_trimmed(denomination, pnl, true)
}

pub(super) fn tracked_trade_intent_text(
    intent: TrackedTradeIntent,
    dir: &str,
    fill_count: usize,
) -> String {
    let intent_text = if intent == TrackedTradeIntent::Unknown && !dir.is_empty() {
        dir.to_string()
    } else {
        intent.label().to_string()
    };

    if fill_count > 1 {
        format!("{intent_text} x{fill_count}")
    } else {
        intent_text
    }
}

fn format_trimmed_number(value: f64, decimals: usize) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }

    trim_decimal_zeros(format!("{value:.decimals$}"))
}

fn format_display_trimmed(
    denomination: &DisplayDenominationContext,
    value: f64,
    signed: bool,
) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }
    let Some(display_value) = denomination.convert_usd_value(value) else {
        return "-".to_string();
    };

    let sign = if display_value < 0.0 {
        "-"
    } else if signed {
        "+"
    } else {
        ""
    };
    let abs = display_value.abs();
    if abs >= 1_000_000.0 {
        denomination.format_active_amount(
            sign,
            format!("{}M", format_trimmed_number(abs / 1_000_000.0, 2)),
        )
    } else {
        denomination
            .format_active_amount(sign, trim_decimal_zeros(format_decimal_with_commas(abs, 2)))
    }
}

fn trim_decimal_zeros(value: String) -> String {
    let Some((whole, fraction)) = value.rsplit_once('.') else {
        return value;
    };
    let trimmed = fraction.trim_end_matches('0');
    if trimmed.is_empty() {
        whole.to_string()
    } else {
        format!("{whole}.{trimmed}")
    }
}

#[cfg(test)]
mod tests;

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
        denomination.format_active_amount(
            sign,
            trim_decimal_zeros(format_decimal_with_commas(abs, 2)),
        )
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
mod tests {
    use super::*;
    use crate::config::DisplayDenominationConfig;
    use std::collections::HashMap;

    #[test]
    fn tracked_trade_numbers_drop_empty_decimal_places() {
        assert_eq!(tracked_trade_size_label(2.0), "2");
        assert_eq!(tracked_trade_price_label(12_345.0), "12345");
        assert_eq!(tracked_trade_fee_label(1.0, "USDC"), "1 USDC");
    }

    #[test]
    fn tracked_trade_numbers_keep_meaningful_decimal_places() {
        assert_eq!(tracked_trade_size_label(2.5), "2.5");
        assert_eq!(tracked_trade_price_label(12_345.6789), "12345.6789");
        assert_eq!(tracked_trade_fee_label(0.0123, "USDC"), "0.0123 USDC");
    }

    #[test]
    fn tracked_trade_usd_values_trim_zero_cents() {
        let denomination = DisplayDenominationContext::default();
        assert_eq!(
            tracked_trade_notional_label(&denomination, 12_000.0),
            "$12,000"
        );
        assert_eq!(tracked_trade_pnl_label(&denomination, 12.0), "+$12");
        assert_eq!(tracked_trade_pnl_label(&denomination, -12.5), "-$12.5");
        assert_eq!(
            tracked_trade_notional_label(&denomination, 1_500_000.0),
            "$1.5M"
        );
    }

    #[test]
    fn tracked_trade_hype_values_suffix_unit() {
        let denomination = DisplayDenominationContext::from_mids(
            DisplayDenominationConfig::hype(),
            &HashMap::from([("HYPE".to_string(), 25.0)]),
            &HashMap::from([("HYPE".to_string(), 1_000)]),
            1_000,
        );

        assert_eq!(
            tracked_trade_notional_label(&denomination, 12_500.0),
            "500 HYPE"
        );
        assert_eq!(
            tracked_trade_pnl_label(&denomination, -2_500.0),
            "-100 HYPE"
        );
    }
}

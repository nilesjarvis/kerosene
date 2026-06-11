use crate::helpers::{format_usd, parse_positive_number, positive_finite_value};

// ---------------------------------------------------------------------------
// Size Input Calculations
// ---------------------------------------------------------------------------

pub(in crate::order_views::inputs::size) fn parse_positive_finite(value: &str) -> Option<f64> {
    parse_positive_number(value)
}

pub(in crate::order_views::inputs::size) fn order_notional_text(
    quantity_is_usd: bool,
    symbol_display: &str,
    symbol_is_outcome: bool,
    parsed_qty: Option<f64>,
    parsed_price: Option<f64>,
) -> (Option<f64>, String) {
    let Some(parsed_qty) = parsed_qty else {
        return (None, String::new());
    };

    if quantity_is_usd {
        let coin_text = parsed_price
            .and_then(|price| {
                let coin_val = parsed_qty / price;
                positive_finite_value(coin_val)
            })
            .map(|coin_val| {
                if symbol_is_outcome {
                    format!("\u{2248} {coin_val:.0} {symbol_display}")
                } else {
                    format!("\u{2248} {coin_val:.4} {symbol_display}")
                }
            })
            .unwrap_or_default();
        (Some(parsed_qty), coin_text)
    } else {
        let Some(parsed_price) = parsed_price else {
            return (None, String::new());
        };
        let Some(notional) = positive_finite_value(parsed_qty * parsed_price) else {
            return (None, String::new());
        };
        (
            Some(notional),
            format!("\u{2248} {}", format_usd(&format!("{notional:.2}"))),
        )
    }
}

pub(in crate::order_views::inputs::size) fn denomination_label(
    order_quantity_is_usd: bool,
    active_is_outcome: bool,
    outcome_quote_symbol: &str,
) -> String {
    if active_is_outcome {
        if order_quantity_is_usd {
            outcome_quote_symbol.to_string()
        } else {
            "CONTRACTS".to_string()
        }
    } else if order_quantity_is_usd {
        "USD".to_string()
    } else {
        "COIN".to_string()
    }
}

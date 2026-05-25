use crate::helpers::{format_usd, parse_positive_number, positive_finite_value};

// ---------------------------------------------------------------------------
// Size Input Calculations
// ---------------------------------------------------------------------------

pub(in crate::order_views::inputs::size) fn parse_positive_finite(value: &str) -> Option<f64> {
    parse_positive_number(value)
}

pub(in crate::order_views::inputs::size) fn order_notional_text(
    quantity_is_usd: bool,
    active_symbol: &str,
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
                let mut search_coin = active_symbol;
                if let Some((_, suffix)) = search_coin.split_once(':') {
                    search_coin = suffix;
                }
                format!("\u{2248} {coin_val:.4} {search_coin}")
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
) -> &'static str {
    if active_is_outcome {
        if order_quantity_is_usd {
            "USDH"
        } else {
            "CONTRACTS"
        }
    } else if order_quantity_is_usd {
        "USD"
    } else {
        "COIN"
    }
}

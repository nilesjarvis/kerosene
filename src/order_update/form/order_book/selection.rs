use crate::helpers::{parse_number, positive_finite_value};
use crate::market_state::{OrderBookInstance, OrderBookSymbolMode};

use std::fmt;

// ---------------------------------------------------------------------------
// Order Book Selection Planning
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
pub(super) struct OrderBookPriceSelection {
    pub(super) selected_price: String,
    pub(super) target_symbol: String,
}

impl fmt::Debug for OrderBookPriceSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderBookPriceSelection")
            .field("selected_price", &"<redacted>")
            .field("target_symbol", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OrderBookPriceSelectionError {
    InvalidPrice,
    Unavailable,
}

pub(super) fn order_book_price_selection(
    inst: Option<&OrderBookInstance>,
    active_symbol: &str,
    price: &str,
) -> Result<OrderBookPriceSelection, OrderBookPriceSelectionError> {
    let selected_price = price.trim().to_string();
    if !valid_selected_order_book_price(&selected_price) {
        return Err(OrderBookPriceSelectionError::InvalidPrice);
    }

    let Some(inst) = inst else {
        return Err(OrderBookPriceSelectionError::Unavailable);
    };

    let target_symbol = match &inst.mode {
        OrderBookSymbolMode::Active => active_symbol.to_string(),
        OrderBookSymbolMode::Fixed(symbol) => {
            let symbol = symbol.trim();
            if symbol.is_empty() {
                return Err(OrderBookPriceSelectionError::Unavailable);
            }
            symbol.to_string()
        }
    };

    Ok(OrderBookPriceSelection {
        selected_price,
        target_symbol,
    })
}

fn valid_selected_order_book_price(price: &str) -> bool {
    parse_number(price)
        .and_then(positive_finite_value)
        .is_some()
}

#[cfg(test)]
mod tests;

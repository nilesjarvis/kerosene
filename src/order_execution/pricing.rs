use crate::app_state::TradingTerminal;
#[cfg(test)]
pub(crate) use crate::config::MAX_MARKET_SLIPPAGE_PCT;
pub(crate) use crate::config::{DEFAULT_MARKET_SLIPPAGE_PCT, normalize_market_slippage_pct};
use crate::signing::{float_to_wire, round_price};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Market Price Helpers
// ---------------------------------------------------------------------------

pub(crate) fn market_slippage_fraction(value: f64) -> f64 {
    normalize_market_slippage_pct(value).unwrap_or(DEFAULT_MARKET_SLIPPAGE_PCT) / 100.0
}

impl TradingTerminal {
    pub(crate) fn market_slippage_fraction(&self) -> f64 {
        market_slippage_fraction(self.market_slippage_pct)
    }
}

pub(super) fn slipped_market_price(mid: f64, is_buy: bool, slippage: f64) -> f64 {
    if is_buy {
        mid * (1.0 + slippage)
    } else {
        mid * (1.0 - slippage)
    }
}

pub(crate) fn rounded_market_price(
    mid: f64,
    is_buy: bool,
    slippage: f64,
    sz_decimals: u32,
    is_spot: bool,
) -> f64 {
    round_price(
        slipped_market_price(mid, is_buy, slippage),
        sz_decimals,
        is_spot,
    )
}

pub(super) fn wire_market_price(
    mid: f64,
    is_buy: bool,
    slippage: f64,
    sz_decimals: u32,
    is_spot: bool,
) -> String {
    float_to_wire(rounded_market_price(
        mid,
        is_buy,
        slippage,
        sz_decimals,
        is_spot,
    ))
}

#[cfg(test)]
pub(super) fn wire_rounded_price(price: f64, sz_decimals: u32, is_spot: bool) -> String {
    float_to_wire(round_price(price, sz_decimals, is_spot))
}

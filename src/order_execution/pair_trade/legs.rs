use super::super::pricing::wire_market_price;
use crate::signing::float_to_wire;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Pair Leg Construction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(super) struct PairLegOrder {
    pub(super) coin: String,
    pub(super) asset: u32,
    pub(super) is_buy: bool,
    pub(super) price: String,
    pub(super) size: String,
}

pub(super) fn build_pair_leg_order(
    coin: String,
    asset: u32,
    sz_decimals: u32,
    mid: f64,
    notional: f64,
    is_buy: bool,
    slippage: f64,
) -> Option<PairLegOrder> {
    let quantity = notional / mid;
    if !quantity.is_finite() || quantity <= 0.0 || !slippage.is_finite() || slippage < 0.0 {
        return None;
    }

    Some(PairLegOrder {
        coin,
        asset,
        is_buy,
        price: wire_market_price(mid, is_buy, slippage, sz_decimals, false),
        size: float_to_wire(quantity),
    })
}

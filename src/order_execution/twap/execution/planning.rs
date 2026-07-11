use super::super::helpers::twap_ioc_limit_price;
use crate::api::OrderBook;
use crate::helpers::format_price;
use crate::signing::float_to_wire;
use crate::twap_state::{
    MIN_EXCHANGE_ORDER_NOTIONAL_USD, TwapEventKind, twap_limit_price_for_slice,
    twap_order_notional_meets_minimum,
};
use std::fmt;

// ---------------------------------------------------------------------------
// TWAP Slice Planning
// ---------------------------------------------------------------------------

pub(super) struct TwapPlannedSliceSkip {
    pub(super) kind: TwapEventKind,
    pub(super) message: String,
    pub(super) is_error: bool,
}

impl fmt::Debug for TwapPlannedSliceSkip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TwapPlannedSliceSkip")
            .field("kind", &self.kind)
            .field("message", &"<redacted>")
            .field("is_error", &self.is_error)
            .finish()
    }
}

pub(super) fn validate_twap_planned_slice(
    book: &OrderBook,
    is_buy: bool,
    planned_size: f64,
    min_price: f64,
    max_price: f64,
    sz_decimals: u32,
    is_spot: bool,
) -> Result<f64, TwapPlannedSliceSkip> {
    let Some(raw_limit_price) =
        twap_limit_price_for_slice(book, is_buy, planned_size, min_price, max_price)
    else {
        return Err(TwapPlannedSliceSkip {
            kind: TwapEventKind::SkippedRange,
            message: format!(
                "TWAP slice skipped: book cannot fill {} inside {}-{}",
                float_to_wire(planned_size),
                format_price(min_price),
                format_price(max_price)
            ),
            is_error: false,
        });
    };

    let Some(limit_price) = twap_ioc_limit_price(
        raw_limit_price,
        is_buy,
        sz_decimals,
        is_spot,
        min_price,
        max_price,
    ) else {
        return Err(TwapPlannedSliceSkip {
            kind: TwapEventKind::SkippedRange,
            message: "TWAP slice skipped: rounded IOC price would no longer cross inside range"
                .to_string(),
            is_error: false,
        });
    };

    if !twap_order_notional_meets_minimum(planned_size, limit_price) {
        return Err(TwapPlannedSliceSkip {
            kind: TwapEventKind::SkippedMinimum,
            message: format!(
                concat!(
                    "TWAP slice skipped: child notional ${:.2} is below ",
                    "Hyperliquid's ${:.0} minimum"
                ),
                planned_size * limit_price,
                MIN_EXCHANGE_ORDER_NOTIONAL_USD
            ),
            is_error: true,
        });
    }

    Ok(limit_price)
}

#[cfg(test)]
mod tests;

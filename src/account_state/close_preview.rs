use crate::api::OrderBook;
use crate::read_data_provider::MarketDataSourceContext;
use std::time::{Duration, Instant};

pub(crate) const CLOSE_PREVIEW_MAX_AGE: Duration = Duration::from_secs(10);

/// Only the hovered position owns a depth subscription. The generation also
/// distinguishes leaving and re-entering the same cell from its old stream.
#[derive(Default)]
pub(crate) struct PositionClosePreview {
    pub(crate) generation: u64,
    pub(crate) coin: Option<String>,
    pub(crate) book: Option<(OrderBook, MarketDataSourceContext, Instant)>,
    pub(crate) fee_multiplier: Option<Result<f64, String>>,
    pub(crate) now_ms: u64,
}

impl PositionClosePreview {
    pub(crate) fn clear(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.coin = None;
        self.book = None;
        self.fee_multiplier = None;
        self.now_ms = 0;
    }

    pub(crate) fn fresh_book(&self, now: Instant) -> Option<&OrderBook> {
        let (book, _, received_at) = self.book.as_ref()?;
        // The UI clock can precede the latest received tick by up to a second.
        (now.saturating_duration_since(*received_at) <= CLOSE_PREVIEW_MAX_AGE).then_some(book)
    }
}

pub(crate) struct CloseEstimate {
    pub(crate) average_price: f64,
    pub(crate) gross_pnl: f64,
    pub(crate) exit_fee: f64,
    pub(crate) net_pnl: f64,
    pub(crate) levels_used: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CloseEstimateError {
    InvalidInputs,
    InvalidBook,
    InsufficientDepth,
    SlippageLimit,
}

/// Walk the bids for a long and asks for a short, consuming the final level
/// only as far as needed. Never price the unobserved remainder of a position.
pub(crate) fn estimate_position_close(
    book: &OrderBook,
    signed_size: f64,
    entry_price: f64,
    fee_rate: f64,
    limit_price: f64,
) -> Result<CloseEstimate, CloseEstimateError> {
    if !signed_size.is_finite()
        || signed_size.abs() <= 1e-12
        || !entry_price.is_finite()
        || entry_price <= 0.0
        || !fee_rate.is_finite()
        || fee_rate < 0.0
        || !limit_price.is_finite()
        || limit_price <= 0.0
    {
        return Err(CloseEstimateError::InvalidInputs);
    }
    let is_buy = signed_size < 0.0;
    let levels = if is_buy { &book.asks } else { &book.bids };
    let size = signed_size.abs();
    let mut remaining = size;
    let mut notional = 0.0;
    let mut previous_price = None;
    for (level_index, level) in levels.iter().enumerate() {
        if !level.px.is_finite()
            || level.px <= 0.0
            || !level.sz.is_finite()
            || level.sz <= 0.0
            || previous_price.is_some_and(|previous| {
                if is_buy {
                    level.px < previous
                } else {
                    level.px > previous
                }
            })
        {
            return Err(CloseEstimateError::InvalidBook);
        }
        if (is_buy && level.px > limit_price) || (!is_buy && level.px < limit_price) {
            return Err(CloseEstimateError::SlippageLimit);
        }
        let filled = remaining.min(level.sz);
        notional += filled * level.px;
        remaining -= filled;
        previous_price = Some(level.px);
        if remaining <= size * 1e-12 {
            let average_price = notional / size;
            let gross_pnl = (average_price - entry_price) * signed_size;
            let exit_fee = notional * fee_rate;
            let net_pnl = gross_pnl - exit_fee;
            if [average_price, gross_pnl, exit_fee, net_pnl]
                .iter()
                .any(|value| !value.is_finite())
            {
                return Err(CloseEstimateError::InvalidInputs);
            }
            return Ok(CloseEstimate {
                average_price,
                gross_pnl,
                exit_fee,
                net_pnl,
                levels_used: level_index + 1,
            });
        }
    }
    Err(CloseEstimateError::InsufficientDepth)
}

#[cfg(test)]
mod tests;

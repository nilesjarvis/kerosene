use std::time::Duration;

mod fills;
mod ids;
mod metrics;
mod model;
mod order;
mod planning;

pub(crate) use self::fills::twap_response_fill_summary;
pub(crate) use self::ids::twap_child_cloid;
pub(crate) use self::metrics::twap_weighted_average_fill_price;
pub(crate) use self::model::{
    TwapBookSnapshot, TwapChildOrder, TwapChildStatus, TwapEvent, TwapEventKind, TwapOrder,
    TwapOrderForm, TwapOrderInit, TwapPauseReason, TwapPendingOp, TwapPendingSlice, TwapStatus,
};
pub(crate) use self::planning::{
    parse_twap_duration_minutes, parse_twap_slice_count, quantize_twap_slice_size,
    twap_aggregate_schedule_has_capacity, twap_aggregate_slice_rate, twap_limit_price_for_slice,
    twap_min_quantized_child_notional, twap_order_notional_meets_minimum, twap_required_slice_rate,
    twap_target_size_from_quantity, validate_twap_interval,
};

// ---------------------------------------------------------------------------
// TWAP Advanced Order State
// ---------------------------------------------------------------------------

pub(crate) const MAX_ACTIVE_ADVANCED_ORDERS: usize = 8;
pub(crate) const TWAP_DEFAULT_DURATION_MINUTES: &str = "30";
pub(crate) const TWAP_DEFAULT_SLICES: &str = "10";
pub(crate) const TWAP_MAX_SLICES: u32 = 100;
pub(crate) const TWAP_MIN_DURATION: Duration = Duration::from_secs(60);
pub(crate) const TWAP_MAX_DURATION: Duration = Duration::from_secs(24 * 60 * 60);
pub(crate) const TWAP_MIN_INTERVAL: Duration = Duration::from_secs(5);
pub(crate) const TWAP_BOOK_STALE_AFTER: Duration = Duration::from_secs(2);
pub(crate) const ADVANCED_ORDER_GLOBAL_EXCHANGE_INTERVAL: Duration = Duration::from_millis(250);
pub(crate) const MIN_EXCHANGE_ORDER_NOTIONAL_USD: f64 = 10.0;
pub(crate) const TWAP_MAX_AGGREGATE_SLICE_RATE: f64 = 1.0;
pub(crate) const TWAP_RETRY_BASE_DELAY: Duration = Duration::from_secs(2);
pub(crate) const TWAP_RETRY_MAX_DELAY: Duration = Duration::from_secs(60);
pub(crate) const TWAP_MAX_RETRY_ATTEMPTS: u32 = 5;
pub(crate) const TWAP_MAX_UNEXPECTED_CANCEL_RETRIES: u32 = 5;
/// Bounded time to wait for an `account.fills` sync to surface a child the
/// exchange has reported as `filled`. The exchange's `orderStatus` is
/// authoritative for "did this child fill", but `account.fills` is what
/// advances `filled_size` and lets the next slice schedule. If the fills
/// sync never catches up — degraded indexer, network drop, etc. — without
/// this timeout the TWAP would sit paused forever with `status_check_cloid`
/// set, blocking `can_schedule_at`.
pub(crate) const TWAP_RECONCILIATION_TIMEOUT: Duration = Duration::from_secs(60);

const TWAP_RANDOM_JITTER: f64 = 0.20;
const TWAP_EVENT_LIMIT: usize = 200;

#[cfg(test)]
mod tests;

use crate::account::UserFill;
use crate::api::OrderBook;
use crate::signing::ExchangeResponse;
use iced::window;
use sha3::{Digest, Keccak256};
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

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

#[derive(Debug, Clone)]
pub(crate) struct TwapOrderForm {
    pub(crate) duration_minutes: String,
    pub(crate) slices: String,
    pub(crate) min_price: String,
    pub(crate) max_price: String,
    pub(crate) randomize: bool,
}

impl Default for TwapOrderForm {
    fn default() -> Self {
        Self {
            duration_minutes: TWAP_DEFAULT_DURATION_MINUTES.to_string(),
            slices: TWAP_DEFAULT_SLICES.to_string(),
            min_price: String::new(),
            max_price: String::new(),
            randomize: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TwapStatus {
    Running,
    WaitingForMarket,
    Paused,
    Stopping,
    Stopped,
    Completed,
    CompletedPartial,
    Error,
}

impl TwapStatus {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Running => "Running",
            Self::WaitingForMarket => "Waiting",
            Self::Paused => "Paused",
            Self::Stopping => "Stopping",
            Self::Stopped => "Stopped",
            Self::Completed => "Done",
            Self::CompletedPartial => "Partial",
            Self::Error => "Error",
        }
    }

    pub(crate) fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Stopped | Self::Completed | Self::CompletedPartial | Self::Error
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TwapPauseReason {
    StaleMarketData,
    RateLimited,
    NetworkError,
    StatusUnknown,
    UnexpectedResting,
}

impl TwapPauseReason {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::StaleMarketData => "Stale market data",
            Self::RateLimited => "Rate limited",
            Self::NetworkError => "Network error",
            Self::StatusUnknown => "Checking status",
            Self::UnexpectedResting => "Canceling child",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TwapChildStatus {
    Pending,
    Retrying,
    Filled,
    NoFill,
    Rejected,
    UnexpectedResting,
    UnexpectedRestingCancelled,
    AwaitingReconciliation,
    StatusUnknown,
}

impl TwapChildStatus {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Retrying => "Retrying",
            Self::Filled => "Filled",
            Self::NoFill => "No fill",
            Self::Rejected => "Rejected",
            Self::UnexpectedResting => "Resting",
            Self::UnexpectedRestingCancelled => "Canceled",
            Self::AwaitingReconciliation => "Reconciling",
            Self::StatusUnknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TwapEventKind {
    Started,
    Placed,
    Filled,
    SkippedRange,
    SkippedMinimum,
    Paused,
    Retrying,
    Reconciled,
    Rejected,
    Stopped,
    Completed,
    Error,
}

#[derive(Debug, Clone)]
pub(crate) struct TwapEvent {
    pub(crate) at: Instant,
    pub(crate) kind: TwapEventKind,
    pub(crate) message: String,
    pub(crate) is_error: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TwapPendingSlice {
    pub(crate) index: u32,
    pub(crate) planned_size: f64,
    pub(crate) limit_price: f64,
    pub(crate) cloid: String,
    pub(crate) retry_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TwapPendingOp {
    Place(TwapPendingSlice),
    CancelUnexpectedResting {
        oid: Option<u64>,
        cloid: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct TwapBookSnapshot {
    pub(crate) book: OrderBook,
    pub(crate) updated_at: Instant,
}

#[derive(Debug, Clone)]
pub(crate) struct TwapChildOrder {
    pub(crate) index: u32,
    pub(crate) requested_at: Instant,
    pub(crate) planned_size: f64,
    pub(crate) limit_price: f64,
    pub(crate) oid: Option<u64>,
    pub(crate) cloid: Option<String>,
    pub(crate) status: TwapChildStatus,
    pub(crate) exchange_summary: String,
    pub(crate) filled_size: f64,
    pub(crate) avg_price: Option<f64>,
    pub(crate) fee: f64,
    pub(crate) retry_count: u32,
}

#[derive(Clone)]
pub(crate) struct TwapOrder {
    pub(crate) id: u64,
    pub(crate) coin: String,
    pub(crate) display_coin: String,
    pub(crate) account_address: String,
    pub(crate) agent_key: Zeroizing<String>,
    pub(crate) is_buy: bool,
    pub(crate) target_size: f64,
    pub(crate) remaining_size: f64,
    pub(crate) filled_size: f64,
    pub(crate) asset: u32,
    pub(crate) sz_decimals: u32,
    pub(crate) is_spot: bool,
    pub(crate) reduce_only: bool,
    pub(crate) min_price: f64,
    pub(crate) max_price: f64,
    pub(crate) randomize: bool,
    pub(crate) random_seed: u64,
    pub(crate) duration: Duration,
    pub(crate) slice_count: u32,
    pub(crate) slices_attempted: u32,
    pub(crate) slices_sent: u32,
    pub(crate) started_at: Instant,
    pub(crate) started_at_ms: u64,
    pub(crate) ends_at: Instant,
    pub(crate) next_slice_due: Instant,
    pub(crate) pending_op: Option<TwapPendingOp>,
    pub(crate) latest_book: Option<TwapBookSnapshot>,
    pub(crate) status: TwapStatus,
    pub(crate) pause_reason: Option<TwapPauseReason>,
    pub(crate) paused_until: Option<Instant>,
    pub(crate) retry_slice: Option<TwapPendingSlice>,
    pub(crate) status_check_cloid: Option<String>,
    pub(crate) status_check_retries: u32,
    /// Deadline by which `account.fills` must observe a child the exchange
    /// already reported as `filled`. `None` when the TWAP is not awaiting
    /// reconciliation. Set when entering `AwaitingReconciliation`, cleared
    /// when fills sync catches up — or when the timeout fires and the TWAP
    /// transitions to terminal error.
    pub(crate) reconciliation_deadline: Option<Instant>,
    pub(crate) cancel_retries: u32,
    pub(crate) stop_requested: bool,
    pub(crate) stop_reason: Option<(String, bool)>,
    pub(crate) child_orders: Vec<TwapChildOrder>,
    pub(crate) events: Vec<TwapEvent>,
    pub(crate) window_id: Option<window::Id>,
}

impl TwapOrder {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        id: u64,
        coin: String,
        display_coin: String,
        account_address: String,
        agent_key: Zeroizing<String>,
        is_buy: bool,
        target_size: f64,
        asset: u32,
        sz_decimals: u32,
        is_spot: bool,
        reduce_only: bool,
        min_price: f64,
        max_price: f64,
        randomize: bool,
        duration: Duration,
        slice_count: u32,
        now: Instant,
        started_at_ms: u64,
    ) -> Self {
        let mut order = Self {
            id,
            coin,
            display_coin,
            account_address,
            agent_key,
            is_buy,
            target_size,
            remaining_size: target_size,
            filled_size: 0.0,
            asset,
            sz_decimals,
            is_spot,
            reduce_only,
            min_price,
            max_price,
            randomize,
            random_seed: twap_seed(id, now),
            duration,
            slice_count,
            slices_attempted: 0,
            slices_sent: 0,
            started_at: now,
            started_at_ms,
            ends_at: now + duration,
            next_slice_due: now,
            pending_op: None,
            latest_book: None,
            status: TwapStatus::WaitingForMarket,
            pause_reason: None,
            paused_until: None,
            retry_slice: None,
            status_check_cloid: None,
            status_check_retries: 0,
            reconciliation_deadline: None,
            cancel_retries: 0,
            stop_requested: false,
            stop_reason: None,
            child_orders: Vec::new(),
            events: Vec::new(),
            window_id: None,
        };
        order.push_event(TwapEventKind::Started, "TWAP started".to_string(), false);
        order
    }

    pub(crate) fn side_label(&self) -> &'static str {
        if self.is_buy { "BUY" } else { "SELL" }
    }

    pub(crate) fn can_schedule(&self) -> bool {
        !self.status.is_terminal() && !self.stop_requested && self.pending_op.is_none()
    }

    pub(crate) fn can_schedule_at(&self, now: Instant) -> bool {
        self.can_schedule()
            && self.status_check_cloid.is_none()
            && self.next_slice_due <= now
            && self.paused_until.is_none_or(|until| until <= now)
    }

    pub(crate) fn pause(
        &mut self,
        reason: TwapPauseReason,
        paused_until: Option<Instant>,
        message: String,
        is_error: bool,
    ) {
        self.status = TwapStatus::Paused;
        self.pause_reason = Some(reason);
        self.paused_until = paused_until;
        if let Some(until) = paused_until {
            self.next_slice_due = until;
        }
        self.push_event(TwapEventKind::Paused, message, is_error);
    }

    pub(crate) fn clear_pause(&mut self) {
        self.pause_reason = None;
        self.paused_until = None;
        self.status_check_retries = 0;
        if !self.status.is_terminal() && !self.stop_requested && self.pending_op.is_none() {
            self.status = TwapStatus::WaitingForMarket;
        }
    }

    /// Pure predicate so the timeout policy can be unit-tested against
    /// arbitrary deadlines without constructing a full TwapOrder.
    pub(crate) fn reconciliation_timed_out(deadline: Option<Instant>, now: Instant) -> bool {
        deadline.is_some_and(|d| now >= d)
    }

    pub(crate) fn retry_delay(retry_count: u32) -> Duration {
        let exponent = retry_count.saturating_sub(1).min(8);
        let multiplier = 1_u32.checked_shl(exponent).unwrap_or(u32::MAX);
        TWAP_RETRY_BASE_DELAY
            .saturating_mul(multiplier)
            .min(TWAP_RETRY_MAX_DELAY)
    }

    pub(crate) fn has_status_unknown_child(&self) -> bool {
        self.child_orders.iter().any(|child| {
            matches!(
                child.status,
                TwapChildStatus::StatusUnknown | TwapChildStatus::AwaitingReconciliation
            )
        })
    }

    pub(crate) fn progress_fraction(&self) -> f64 {
        if self.target_size.is_finite() && self.target_size > 0.0 {
            (self.filled_size / self.target_size).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    pub(crate) fn push_event(&mut self, kind: TwapEventKind, message: String, is_error: bool) {
        self.events.push(TwapEvent {
            at: Instant::now(),
            kind,
            message,
            is_error,
        });
        if self.events.len() > TWAP_EVENT_LIMIT {
            let excess = self.events.len().saturating_sub(TWAP_EVENT_LIMIT);
            self.events.drain(0..excess);
        }
    }

    pub(crate) fn next_slice_size(&mut self) -> Option<f64> {
        if !self.remaining_size.is_finite() || self.remaining_size <= 0.0 {
            return None;
        }
        let remaining_slots = self.slice_count.saturating_sub(self.slices_attempted);
        if remaining_slots == 0 {
            return None;
        }
        if remaining_slots == 1 {
            return Some(self.remaining_size);
        }

        let base = self.remaining_size / f64::from(remaining_slots);
        let size = if self.randomize {
            let unit = next_random_unit(&mut self.random_seed);
            let factor = 1.0 - TWAP_RANDOM_JITTER + unit * TWAP_RANDOM_JITTER * 2.0;
            base * factor
        } else {
            base
        };
        let max_size = self.remaining_size;
        let size = size.clamp(f64::MIN_POSITIVE, max_size);
        size.is_finite().then_some(size)
    }

    pub(crate) fn schedule_after_attempt(&mut self, now: Instant) {
        if self.remaining_size <= 0.0 {
            self.clear_pause();
            self.status = TwapStatus::Completed;
            self.push_event(
                TwapEventKind::Completed,
                "TWAP completed".to_string(),
                false,
            );
            return;
        }
        let remaining_slots = self.slice_count.saturating_sub(self.slices_attempted);
        if remaining_slots == 0 {
            self.clear_pause();
            self.status = if self.filled_size > 0.0 {
                TwapStatus::CompletedPartial
            } else {
                TwapStatus::Stopped
            };
            let message = if self.filled_size > 0.0 {
                "TWAP ended with unfilled remainder".to_string()
            } else {
                "TWAP ended without fills".to_string()
            };
            self.push_event(TwapEventKind::Completed, message, false);
            return;
        }

        let remaining_time = self.ends_at.saturating_duration_since(now);
        if remaining_time.is_zero() {
            self.next_slice_due = now;
            self.status = TwapStatus::WaitingForMarket;
            return;
        }

        let nominal_delay = remaining_time / remaining_slots;
        let delay = if self.randomize {
            let unit = next_random_unit(&mut self.random_seed);
            let factor = 1.0 - TWAP_RANDOM_JITTER + unit * TWAP_RANDOM_JITTER * 2.0;
            scaled_duration(nominal_delay, factor)
        } else {
            nominal_delay
        };
        let future_min = TWAP_MIN_INTERVAL.saturating_mul(remaining_slots.saturating_sub(1));
        let max_delay = remaining_time.saturating_sub(future_min);
        let delay = clamp_duration(delay, TWAP_MIN_INTERVAL.min(remaining_time), max_delay);
        self.next_slice_due = now + delay;
        self.clear_pause();
        self.status = TwapStatus::WaitingForMarket;
    }

    pub(crate) fn mark_filled(&mut self, filled_size: f64) {
        if !filled_size.is_finite() || filled_size <= 0.0 {
            return;
        }
        self.filled_size = (self.filled_size + filled_size).min(self.target_size);
        self.remaining_size = (self.target_size - self.filled_size).max(0.0);
        if self.remaining_size <= f64::EPSILON {
            self.remaining_size = 0.0;
            self.clear_pause();
            self.status = TwapStatus::Completed;
        }
    }

    pub(crate) fn reconcile_fills(&mut self, fills: &[UserFill]) {
        let had_status_unknown = self.has_status_unknown_child();

        for child in &mut self.child_orders {
            let Some(oid) = child.oid else {
                continue;
            };
            let summary = fill_summary_for_oid(fills, oid);
            if let Some(summary) = summary {
                child.filled_size = child.filled_size.max(summary.filled_size);
                child.avg_price = summary.avg_price.or(child.avg_price);
                child.fee = child.fee.max(summary.fee.abs());
                if child.filled_size > 0.0 && child.status != TwapChildStatus::Rejected {
                    child.status = TwapChildStatus::Filled;
                }
            }
        }

        let reconciled: f64 = self
            .child_orders
            .iter()
            .map(|child| child.filled_size)
            .sum();
        if reconciled.is_finite() && reconciled > self.filled_size {
            self.filled_size = reconciled.min(self.target_size);
            self.remaining_size = (self.target_size - self.filled_size).max(0.0);
        }
        if self.remaining_size <= f64::EPSILON
            && (matches!(
                self.status,
                TwapStatus::Running
                    | TwapStatus::WaitingForMarket
                    | TwapStatus::Paused
                    | TwapStatus::CompletedPartial
            ) || (had_status_unknown && self.status == TwapStatus::Error))
        {
            self.remaining_size = 0.0;
            self.clear_pause();
            self.status = TwapStatus::Completed;
        } else if had_status_unknown && self.status == TwapStatus::Error && self.filled_size > 0.0 {
            self.status = TwapStatus::CompletedPartial;
        } else if self.status == TwapStatus::Paused && !self.has_status_unknown_child() {
            self.clear_pause();
        }
    }
}

impl std::fmt::Debug for TwapOrder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TwapOrder")
            .field("id", &self.id)
            .field("coin", &self.coin)
            .field("display_coin", &self.display_coin)
            .field("account_address", &self.account_address)
            .field("agent_key", &"<redacted>")
            .field("is_buy", &self.is_buy)
            .field("target_size", &self.target_size)
            .field("remaining_size", &self.remaining_size)
            .field("filled_size", &self.filled_size)
            .field("asset", &self.asset)
            .field("sz_decimals", &self.sz_decimals)
            .field("is_spot", &self.is_spot)
            .field("reduce_only", &self.reduce_only)
            .field("min_price", &self.min_price)
            .field("max_price", &self.max_price)
            .field("randomize", &self.randomize)
            .field("duration", &self.duration)
            .field("slice_count", &self.slice_count)
            .field("slices_attempted", &self.slices_attempted)
            .field("slices_sent", &self.slices_sent)
            .field("status", &self.status)
            .field("pause_reason", &self.pause_reason)
            .field("paused_until", &self.paused_until)
            .field("retry_slice", &self.retry_slice)
            .field("status_check_cloid", &self.status_check_cloid)
            .field("status_check_retries", &self.status_check_retries)
            .field("reconciliation_deadline", &self.reconciliation_deadline)
            .field("cancel_retries", &self.cancel_retries)
            .field("stop_requested", &self.stop_requested)
            .finish()
    }
}

pub(crate) fn twap_child_cloid(
    account_address: &str,
    twap_id: u64,
    started_at_ms: u64,
    slice_index: u32,
) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(account_address.as_bytes());
    hasher.update(twap_id.to_be_bytes());
    hasher.update(started_at_ms.to_be_bytes());
    hasher.update(slice_index.to_be_bytes());
    let hash = hasher.finalize();

    let mut cloid = String::with_capacity(34);
    cloid.push_str("0x");
    for byte in hash.iter().take(16) {
        use std::fmt::Write;
        let _ = write!(cloid, "{byte:02x}");
    }
    cloid
}

pub(crate) fn parse_twap_duration_minutes(value: &str) -> Option<Duration> {
    let minutes = value.trim().parse::<f64>().ok()?;
    if !minutes.is_finite() || minutes <= 0.0 {
        return None;
    }
    let duration = Duration::from_secs_f64(minutes * 60.0);
    (duration >= TWAP_MIN_DURATION && duration <= TWAP_MAX_DURATION).then_some(duration)
}

pub(crate) fn parse_twap_slice_count(value: &str) -> Option<u32> {
    let count = value.trim().parse::<u32>().ok()?;
    (count > 0 && count <= TWAP_MAX_SLICES).then_some(count)
}

pub(crate) fn validate_twap_interval(duration: Duration, slice_count: u32) -> bool {
    slice_count > 0 && duration / slice_count >= TWAP_MIN_INTERVAL
}

pub(crate) fn twap_target_size_from_quantity(
    raw_quantity: f64,
    reference_price: Option<f64>,
    quantity_is_usd: bool,
) -> Option<f64> {
    if !raw_quantity.is_finite() || raw_quantity <= 0.0 {
        return None;
    }
    let target_size = if quantity_is_usd {
        let reference_price = reference_price?;
        if !reference_price.is_finite() || reference_price <= 0.0 {
            return None;
        }
        raw_quantity / reference_price
    } else {
        raw_quantity
    };
    (target_size.is_finite() && target_size > 0.0).then_some(target_size)
}

pub(crate) fn twap_required_slice_rate(duration: Duration, slice_count: u32) -> Option<f64> {
    if slice_count == 0 {
        return None;
    }
    let seconds = duration.as_secs_f64();
    if !seconds.is_finite() || seconds <= 0.0 {
        return Some(f64::INFINITY);
    }
    Some(f64::from(slice_count) / seconds)
}

pub(crate) fn twap_aggregate_slice_rate(
    active_slice_rate: f64,
    duration: Duration,
    slice_count: u32,
) -> Option<f64> {
    if !active_slice_rate.is_finite() || active_slice_rate < 0.0 {
        return None;
    }
    let new_rate = twap_required_slice_rate(duration, slice_count)?;
    let total_rate = active_slice_rate + new_rate;
    (total_rate.is_finite() && total_rate >= 0.0).then_some(total_rate)
}

pub(crate) fn twap_aggregate_schedule_has_capacity(
    active_slice_rate: f64,
    duration: Duration,
    slice_count: u32,
) -> bool {
    twap_aggregate_slice_rate(active_slice_rate, duration, slice_count)
        .is_some_and(|rate| rate <= TWAP_MAX_AGGREGATE_SLICE_RATE + f64::EPSILON)
}

pub(crate) fn twap_min_quantized_child_notional(
    target_size: f64,
    slice_count: u32,
    min_price: f64,
    randomize: bool,
    sz_decimals: u32,
) -> Option<f64> {
    if !target_size.is_finite()
        || target_size <= 0.0
        || slice_count == 0
        || !min_price.is_finite()
        || min_price <= 0.0
    {
        return None;
    }
    let base_size = target_size / f64::from(slice_count);
    let min_size = if randomize {
        base_size * 0.8
    } else {
        base_size
    };
    let quantized_size = quantize_twap_slice_size(min_size, target_size, sz_decimals)?;
    let notional = quantized_size * min_price;
    (notional.is_finite() && notional > 0.0).then_some(notional)
}

pub(crate) fn twap_order_notional_meets_minimum(size: f64, price: f64) -> bool {
    size.is_finite()
        && size > 0.0
        && price.is_finite()
        && price > 0.0
        && size * price >= MIN_EXCHANGE_ORDER_NOTIONAL_USD
}

pub(crate) fn quantize_twap_slice_size(
    size: f64,
    remaining_size: f64,
    sz_decimals: u32,
) -> Option<f64> {
    if !size.is_finite() || size <= 0.0 || !remaining_size.is_finite() || remaining_size <= 0.0 {
        return None;
    }
    let decimals = sz_decimals.min(8);
    let factor = 10f64.powi(decimals as i32);
    let max_size = (remaining_size * factor).floor() / factor;
    let quantized = ((size.min(remaining_size)) * factor).floor() / factor;
    let quantized = quantized.min(max_size);
    (quantized.is_finite() && quantized > 0.0).then_some(quantized)
}

pub(crate) fn twap_limit_price_for_slice(
    book: &OrderBook,
    is_buy: bool,
    planned_size: f64,
    min_price: f64,
    max_price: f64,
) -> Option<f64> {
    if !planned_size.is_finite()
        || planned_size <= 0.0
        || !min_price.is_finite()
        || !max_price.is_finite()
        || min_price <= 0.0
        || max_price <= min_price
    {
        return None;
    }

    let levels = if is_buy { &book.asks } else { &book.bids };
    let best = levels.first()?.px;
    if !best.is_finite() || best < min_price || best > max_price {
        return None;
    }

    let mut cumulative_size = 0.0;
    for level in levels {
        let price = level.px;
        let size = level.sz;
        if !price.is_finite() || !size.is_finite() || price <= 0.0 || size <= 0.0 {
            continue;
        }
        if price < min_price || price > max_price {
            break;
        }
        cumulative_size += size;
        if cumulative_size >= planned_size {
            return Some(price);
        }
    }
    None
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ResponseFillSummary {
    pub(crate) oid: Option<u64>,
    pub(crate) filled_size: f64,
    pub(crate) avg_price: Option<f64>,
}

pub(crate) fn twap_response_fill_summary(response: &ExchangeResponse) -> ResponseFillSummary {
    let mut summary = ResponseFillSummary::default();
    let Some(statuses) = response
        .response
        .as_ref()
        .and_then(|inner| inner.data.as_ref())
        .map(|data| data.statuses.as_slice())
    else {
        return summary;
    };

    for status in statuses {
        let Some(filled) = status.get("filled") else {
            continue;
        };
        if summary.oid.is_none() {
            summary.oid = filled.get("oid").and_then(|value| value.as_u64());
        }
        if let Some(size) = filled
            .get("totalSz")
            .and_then(|value| value.as_str())
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|size| size.is_finite() && *size > 0.0)
        {
            summary.filled_size += size;
        }
        if summary.avg_price.is_none() {
            summary.avg_price = filled
                .get("avgPx")
                .and_then(|value| value.as_str())
                .and_then(|value| value.parse::<f64>().ok())
                .filter(|price| price.is_finite() && *price > 0.0);
        }
    }
    summary
}

fn twap_seed(id: u64, now: Instant) -> u64 {
    let nanos = now.elapsed().as_nanos() as u64;
    id.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(nanos)
        .max(1)
}

fn next_random_unit(seed: &mut u64) -> f64 {
    let mut value = (*seed).max(1);
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *seed = value.max(1);
    (value as f64 / u64::MAX as f64).clamp(0.0, 1.0)
}

fn scaled_duration(duration: Duration, factor: f64) -> Duration {
    if !factor.is_finite() || factor <= 0.0 {
        return duration;
    }
    Duration::from_secs_f64(duration.as_secs_f64() * factor)
}

fn clamp_duration(value: Duration, min: Duration, max: Duration) -> Duration {
    if max < min {
        return max;
    }
    value.max(min).min(max)
}

#[derive(Debug, Clone, Copy)]
struct FillSummary {
    filled_size: f64,
    avg_price: Option<f64>,
    fee: f64,
}

fn fill_summary_for_oid(fills: &[UserFill], oid: u64) -> Option<FillSummary> {
    let mut filled_size = 0.0;
    let mut notional = 0.0;
    let mut fee = 0.0;

    for fill in fills.iter().filter(|fill| fill.oid == Some(oid)) {
        let Ok(size) = fill.sz.parse::<f64>() else {
            continue;
        };
        let Ok(price) = fill.px.parse::<f64>() else {
            continue;
        };
        if !size.is_finite() || size <= 0.0 || !price.is_finite() || price <= 0.0 {
            continue;
        }
        filled_size += size;
        notional += size * price;
        if let Ok(parsed_fee) = fill.fee.parse::<f64>()
            && parsed_fee.is_finite()
        {
            fee += parsed_fee.abs();
        }
    }

    if filled_size <= 0.0 {
        return None;
    }
    Some(FillSummary {
        filled_size,
        avg_price: Some(notional / filled_size),
        fee,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        MIN_EXCHANGE_ORDER_NOTIONAL_USD, TWAP_MAX_AGGREGATE_SLICE_RATE,
        TWAP_RECONCILIATION_TIMEOUT, TwapChildOrder, TwapChildStatus, TwapOrder, TwapPauseReason,
        TwapStatus, parse_twap_duration_minutes, parse_twap_slice_count, quantize_twap_slice_size,
        twap_aggregate_schedule_has_capacity, twap_aggregate_slice_rate, twap_child_cloid,
        twap_limit_price_for_slice, twap_min_quantized_child_notional,
        twap_order_notional_meets_minimum, twap_required_slice_rate, twap_response_fill_summary,
        twap_target_size_from_quantity, validate_twap_interval,
    };
    use crate::account::UserFill;
    use crate::api::{BookLevel, OrderBook};
    use crate::signing::ExchangeResponse;
    use std::time::{Duration, Instant};

    fn book(bids: &[(f64, f64)], asks: &[(f64, f64)]) -> OrderBook {
        OrderBook {
            bids: bids
                .iter()
                .map(|(px, sz)| BookLevel { px: *px, sz: *sz })
                .collect(),
            asks: asks
                .iter()
                .map(|(px, sz)| BookLevel { px: *px, sz: *sz })
                .collect(),
        }
    }

    fn user_fill(oid: u64, size: &str, price: &str) -> UserFill {
        UserFill {
            coin: "BTC".to_string(),
            px: price.to_string(),
            sz: size.to_string(),
            side: "B".to_string(),
            time: 1,
            oid: Some(oid),
            dir: "Open Long".to_string(),
            closed_pnl: "0".to_string(),
            fee: "0.01".to_string(),
        }
    }

    #[test]
    fn twap_price_gate_walks_buy_depth_inside_range() {
        let book = book(&[(99.0, 1.0)], &[(100.0, 0.5), (101.0, 0.75)]);
        assert_eq!(
            twap_limit_price_for_slice(&book, true, 1.0, 99.0, 101.0),
            Some(101.0)
        );
        assert_eq!(
            twap_limit_price_for_slice(&book, true, 1.0, 99.0, 100.5),
            None
        );
    }

    #[test]
    fn twap_price_gate_walks_sell_depth_inside_range() {
        let book = book(&[(100.0, 0.25), (99.0, 1.0)], &[(101.0, 1.0)]);
        assert_eq!(
            twap_limit_price_for_slice(&book, false, 1.0, 99.0, 101.0),
            Some(99.0)
        );
        assert_eq!(
            twap_limit_price_for_slice(&book, false, 1.0, 99.5, 101.0),
            None
        );
    }

    #[test]
    fn twap_price_gate_rejects_best_price_outside_hard_range() {
        let book = book(&[(105.0, 1.0)], &[(95.0, 1.0)]);
        assert_eq!(
            twap_limit_price_for_slice(&book, true, 0.5, 99.0, 101.0),
            None
        );
        assert_eq!(
            twap_limit_price_for_slice(&book, false, 0.5, 99.0, 101.0),
            None
        );
    }

    #[test]
    fn randomized_sizes_never_overshoot_target() {
        let now = Instant::now();
        let mut twap = TwapOrder::new(
            1,
            "BTC".to_string(),
            "BTC".to_string(),
            "0xabc".to_string(),
            "key".to_string().into(),
            true,
            10.0,
            0,
            3,
            false,
            false,
            90.0,
            110.0,
            true,
            Duration::from_secs(60),
            10,
            now,
            1_000,
        );
        let mut total = 0.0;
        while twap.slices_attempted < twap.slice_count {
            let slice = twap.next_slice_size().expect("slice should calculate");
            assert!(slice > 0.0);
            assert!(slice <= twap.remaining_size);
            total += slice;
            twap.remaining_size = (twap.remaining_size - slice).max(0.0);
            twap.slices_attempted += 1;
        }
        assert!(total <= 10.0 + 1e-9);
        assert!(twap.remaining_size <= f64::EPSILON);
    }

    #[test]
    fn skipped_slices_roll_size_forward() {
        let now = Instant::now();
        let mut twap = TwapOrder::new(
            1,
            "BTC".to_string(),
            "BTC".to_string(),
            "0xabc".to_string(),
            "key".to_string().into(),
            true,
            9.0,
            0,
            3,
            false,
            false,
            90.0,
            110.0,
            false,
            Duration::from_secs(60),
            3,
            now,
            1_000,
        );
        let first = twap.next_slice_size().expect("first slice");
        assert_eq!(first, 3.0);
        twap.slices_attempted += 1;
        let second = twap.next_slice_size().expect("rolled slice");
        assert_eq!(second, 4.5);
    }

    #[test]
    fn validates_twap_duration_and_interval() {
        let duration = parse_twap_duration_minutes("1").expect("one minute valid");
        assert!(validate_twap_interval(duration, 12));
        assert!(!validate_twap_interval(duration, 13));
        assert!(parse_twap_duration_minutes("0.1").is_none());
        assert!(parse_twap_slice_count("101").is_none());
    }

    #[test]
    fn slice_size_quantization_respects_asset_precision_without_rounding_up() {
        assert_eq!(quantize_twap_slice_size(1.239, 2.0, 2), Some(1.23));
        assert_eq!(quantize_twap_slice_size(1.239, 1.2, 2), Some(1.2));
        assert_eq!(quantize_twap_slice_size(0.9, 0.9, 0), None);
        assert_eq!(quantize_twap_slice_size(1.9, 2.0, 0), Some(1.0));
    }

    #[test]
    fn twap_child_notional_enforces_exchange_minimum() {
        assert!(twap_order_notional_meets_minimum(
            0.1,
            MIN_EXCHANGE_ORDER_NOTIONAL_USD * 100.0
        ));
        assert!(!twap_order_notional_meets_minimum(0.009, 1_000.0));
        assert_eq!(
            twap_min_quantized_child_notional(1.0, 10, 100.0, false, 3),
            Some(10.0)
        );
        let randomized = twap_min_quantized_child_notional(1.0, 10, 100.0, true, 3)
            .expect("randomized child notional should calculate");
        assert!((randomized - 8.0).abs() < 1e-9);
    }

    #[test]
    fn twap_child_notional_uses_quantized_child_size() {
        assert_eq!(
            twap_min_quantized_child_notional(9.9, 10, 11.0, false, 0),
            None
        );
        assert_eq!(
            twap_min_quantized_child_notional(10.9, 10, 10.0, false, 0),
            Some(10.0)
        );
    }

    #[test]
    fn twap_target_size_requires_fresh_reference_for_usd_quantity() {
        assert_eq!(
            twap_target_size_from_quantity(1_000.0, Some(100.0), true),
            Some(10.0)
        );
        assert_eq!(twap_target_size_from_quantity(1_000.0, None, true), None);
        assert_eq!(twap_target_size_from_quantity(2.5, None, false), Some(2.5));
        assert_eq!(
            twap_target_size_from_quantity(1_000.0, Some(0.0), true),
            None
        );
    }

    #[test]
    fn twap_schedule_capacity_accounts_for_active_slice_rate() {
        let one_minute = Duration::from_secs(60);
        assert_eq!(twap_required_slice_rate(one_minute, 12), Some(0.2));
        assert_eq!(
            twap_required_slice_rate(Duration::ZERO, 1),
            Some(f64::INFINITY)
        );
        assert_eq!(
            twap_aggregate_slice_rate(0.8, one_minute, 12),
            Some(TWAP_MAX_AGGREGATE_SLICE_RATE)
        );
        assert!(twap_aggregate_schedule_has_capacity(0.8, one_minute, 12));
        assert!(!twap_aggregate_schedule_has_capacity(0.81, one_minute, 12));
        assert!(!twap_aggregate_schedule_has_capacity(
            f64::NAN,
            one_minute,
            12
        ));
    }

    #[test]
    fn twap_child_cloid_is_stable_128_bit_hex() {
        let first = twap_child_cloid("0xabc", 7, 1_000, 3);
        let second = twap_child_cloid("0xabc", 7, 1_000, 3);
        let different = twap_child_cloid("0xabc", 7, 1_000, 4);

        assert_eq!(first, second);
        assert_ne!(first, different);
        assert_eq!(first.len(), 34);
        assert!(first.starts_with("0x"));
        assert!(first[2..].chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    #[test]
    fn paused_status_check_blocks_scheduling_until_reconciled() {
        let now = Instant::now();
        let mut twap = TwapOrder::new(
            1,
            "BTC".to_string(),
            "BTC".to_string(),
            "0xabc".to_string(),
            "key".to_string().into(),
            true,
            1.0,
            0,
            3,
            false,
            false,
            90.0,
            110.0,
            false,
            Duration::from_secs(60),
            2,
            now,
            1_000,
        );
        twap.pause(
            TwapPauseReason::StatusUnknown,
            Some(now),
            "checking".to_string(),
            true,
        );
        twap.status_check_cloid = Some(twap_child_cloid("0xabc", 1, 1_000, 1));

        assert!(!twap.can_schedule_at(now));

        twap.status_check_cloid = None;
        assert!(twap.can_schedule_at(now));
    }

    #[test]
    fn retry_delay_exponentially_backs_off_and_caps() {
        assert_eq!(TwapOrder::retry_delay(1), Duration::from_secs(2));
        assert_eq!(TwapOrder::retry_delay(2), Duration::from_secs(4));
        assert_eq!(TwapOrder::retry_delay(10), Duration::from_secs(60));
    }

    #[test]
    fn reconciliation_timed_out_predicate_handles_none_and_boundary() {
        let now = Instant::now();

        // No deadline armed — never timed out.
        assert!(!TwapOrder::reconciliation_timed_out(None, now));

        // Deadline in the future — not yet timed out.
        assert!(!TwapOrder::reconciliation_timed_out(
            Some(now + Duration::from_millis(1)),
            now
        ));

        // Deadline exactly at `now` — counts as timed out so the watchdog
        // fires on the first reconcile after expiry rather than one tick
        // later.
        assert!(TwapOrder::reconciliation_timed_out(Some(now), now));

        // Deadline in the past — timed out.
        assert!(TwapOrder::reconciliation_timed_out(
            Some(now - Duration::from_secs(1)),
            now
        ));
    }

    #[test]
    fn reconciliation_timeout_is_long_enough_to_absorb_typical_indexer_lag() {
        // The exchange's `account.fills` endpoint has been observed to lag
        // a few seconds behind `orderStatus` under normal conditions. The
        // timeout must be loose enough that healthy operation doesn't
        // trip the watchdog. 60s is a generous floor — anything shorter
        // would frequently false-positive during minor indexer hiccups.
        const MIN_HEALTHY_TIMEOUT: Duration = Duration::from_secs(30);
        const _: () = {
            // const-evaluated comparison so a future tightening of the
            // constant fails to compile rather than silently producing
            // flaky terminal errors in production.
            assert!(TWAP_RECONCILIATION_TIMEOUT.as_secs() >= MIN_HEALTHY_TIMEOUT.as_secs());
        };
    }

    #[test]
    fn twap_fill_summary_does_not_invent_missing_fill_size() {
        let response: ExchangeResponse = serde_json::from_value(serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [{
                        "filled": {
                            "avgPx": "100",
                            "oid": 77_u64
                        }
                    }]
                }
            }
        }))
        .expect("test exchange response should deserialize");

        assert!(response.is_fully_filled());
        let summary = twap_response_fill_summary(&response);
        assert_eq!(summary.oid, Some(77));
        assert_eq!(summary.filled_size, 0.0);
        assert_eq!(summary.avg_price, Some(100.0));
    }

    #[test]
    fn status_unknown_twap_reconciles_to_partial_or_completed_from_account_fills() {
        let now = Instant::now();
        let mut partial = TwapOrder::new(
            1,
            "BTC".to_string(),
            "BTC".to_string(),
            "0xabc".to_string(),
            "key".to_string().into(),
            true,
            2.0,
            0,
            3,
            false,
            false,
            90.0,
            110.0,
            false,
            Duration::from_secs(60),
            2,
            now,
            1_000,
        );
        partial.status = TwapStatus::Error;
        partial.child_orders.push(TwapChildOrder {
            index: 1,
            requested_at: now,
            planned_size: 1.0,
            limit_price: 100.0,
            oid: Some(42),
            cloid: Some("0x1234567890abcdef1234567890abcdef".to_string()),
            status: TwapChildStatus::StatusUnknown,
            exchange_summary: "status unknown".to_string(),
            filled_size: 0.0,
            avg_price: None,
            fee: 0.0,
            retry_count: 0,
        });

        partial.reconcile_fills(&[user_fill(42, "1.0", "100")]);
        assert_eq!(partial.status, TwapStatus::CompletedPartial);
        assert_eq!(partial.filled_size, 1.0);
        assert_eq!(partial.remaining_size, 1.0);

        let mut completed = partial.clone();
        completed.target_size = 1.0;
        completed.remaining_size = 1.0;
        completed.filled_size = 0.0;
        completed.status = TwapStatus::Error;
        completed.child_orders[0].filled_size = 0.0;
        completed.child_orders[0].status = TwapChildStatus::StatusUnknown;

        completed.reconcile_fills(&[user_fill(42, "1.0", "100")]);
        assert_eq!(completed.status, TwapStatus::Completed);
        assert_eq!(completed.remaining_size, 0.0);
    }
}

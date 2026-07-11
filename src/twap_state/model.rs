use super::{TWAP_DEFAULT_DURATION_MINUTES, TWAP_DEFAULT_SLICES};

use crate::api::OrderBook;
use crate::signing::CapturedAgentKey;

use iced::window;
use std::fmt;
use std::time::{Duration, Instant};

mod status;

pub(crate) use self::status::{TwapChildStatus, TwapPauseReason, TwapStatus};

// ---------------------------------------------------------------------------
// Form State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
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

// ---------------------------------------------------------------------------
// Runtime Model
// ---------------------------------------------------------------------------

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

#[derive(Clone, PartialEq)]
pub(crate) struct TwapPendingSlice {
    pub(crate) index: u32,
    pub(crate) planned_size: f64,
    pub(crate) limit_price: f64,
    pub(crate) cloid: String,
    pub(crate) retry_count: u32,
}

impl fmt::Debug for TwapPendingSlice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TwapPendingSlice")
            .field("index", &self.index)
            .field("planned_size", &"<redacted>")
            .field("limit_price", &"<redacted>")
            .field("cloid", &"<redacted>")
            .field("retry_count", &self.retry_count)
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub(crate) enum TwapPendingOp {
    Place(TwapPendingSlice),
    CancelUnexpectedResting {
        oid: Option<u64>,
        cloid: Option<String>,
    },
}

impl fmt::Debug for TwapPendingOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Place(slice) => f.debug_tuple("Place").field(slice).finish(),
            Self::CancelUnexpectedResting { oid, cloid } => f
                .debug_struct("CancelUnexpectedResting")
                .field("has_oid", &oid.is_some())
                .field("has_cloid", &cloid.is_some())
                .finish(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TwapBookSnapshot {
    pub(crate) book: OrderBook,
    pub(crate) updated_at: Instant,
}

#[derive(Clone)]
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

impl fmt::Debug for TwapChildOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TwapChildOrder")
            .field("index", &self.index)
            .field("requested_at", &self.requested_at)
            .field("planned_size", &"<redacted>")
            .field("limit_price", &"<redacted>")
            .field("has_oid", &self.oid.is_some())
            .field("has_cloid", &self.cloid.is_some())
            .field("status", &self.status)
            .field("exchange_summary", &"<redacted>")
            .field("filled_size", &"<redacted>")
            .field("avg_price", &"<redacted>")
            .field("fee", &"<redacted>")
            .field("retry_count", &self.retry_count)
            .finish()
    }
}

#[derive(Clone)]
pub(crate) struct TwapOrder {
    pub(crate) id: u64,
    pub(crate) coin: String,
    pub(crate) display_coin: String,
    pub(crate) account_address: String,
    pub(crate) agent_key: CapturedAgentKey,
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
    /// Exact retry attempt currently owns the in-flight CLOID status task.
    /// `status_check_cloid` can remain set while account fills reconcile after
    /// that task completes, so task ownership must be tracked independently.
    pub(crate) status_check_pending_attempt: Option<u32>,
    pub(crate) status_check_retries: u32,
    pub(crate) account_reconciliation_retries: u32,
    /// Deadline by which `account.fills` must reconcile a child whose exchange
    /// status is not yet safe to consume. `None` when the TWAP is not awaiting
    /// reconciliation. Set when entering a reconciliation child state, cleared
    /// when a complete fill snapshot resolves it, or when the timeout fires and
    /// the TWAP transitions to terminal error.
    pub(crate) reconciliation_deadline: Option<Instant>,
    pub(crate) cancel_retries: u32,
    pub(crate) stop_requested: bool,
    pub(crate) stop_reason: Option<(String, bool)>,
    pub(crate) child_orders: Vec<TwapChildOrder>,
    pub(crate) events: Vec<TwapEvent>,
    pub(crate) window_id: Option<window::Id>,
}

pub(crate) struct TwapOrderInit {
    pub(crate) id: u64,
    pub(crate) coin: String,
    pub(crate) display_coin: String,
    pub(crate) account_address: String,
    pub(crate) agent_key: CapturedAgentKey,
    pub(crate) is_buy: bool,
    pub(crate) target_size: f64,
    pub(crate) asset: u32,
    pub(crate) sz_decimals: u32,
    pub(crate) is_spot: bool,
    pub(crate) reduce_only: bool,
    pub(crate) min_price: f64,
    pub(crate) max_price: f64,
    pub(crate) randomize: bool,
    pub(crate) duration: Duration,
    pub(crate) slice_count: u32,
    pub(crate) now: Instant,
    pub(crate) started_at_ms: u64,
}

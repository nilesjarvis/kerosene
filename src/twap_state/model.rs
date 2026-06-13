use super::{TWAP_DEFAULT_DURATION_MINUTES, TWAP_DEFAULT_SLICES};

use crate::api::OrderBook;
use crate::signing::CapturedAgentKey;

use iced::window;
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
    pub(crate) status_check_retries: u32,
    pub(crate) account_reconciliation_retries: u32,
    /// Deadline by which `account.fills` must observe a child the exchange
    /// already reported as `filled`. `None` when the TWAP is not awaiting
    /// reconciliation. Set when entering `AwaitingReconciliation`, cleared
    /// when fills sync catches up, or when the timeout fires and the TWAP
    /// transitions to terminal error.
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

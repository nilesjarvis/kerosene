use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Advanced Order History Model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum AdvancedOrderHistoryKind {
    Chase,
    Twap,
}

impl AdvancedOrderHistoryKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Chase => "CHASE",
            Self::Twap => "TWAP",
        }
    }
}

fn default_history_kind() -> AdvancedOrderHistoryKind {
    AdvancedOrderHistoryKind::Twap
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct AdvancedOrderHistoryLog {
    #[serde(default)]
    pub(crate) elapsed_ms: u64,
    #[serde(default)]
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) message: String,
    #[serde(default)]
    pub(crate) is_error: bool,
}

impl fmt::Debug for AdvancedOrderHistoryLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AdvancedOrderHistoryLog")
            .field("elapsed_ms", &"<redacted>")
            .field("kind", &"<redacted>")
            .field("message", &"<redacted>")
            .field("is_error", &self.is_error)
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct AdvancedOrderHistoryChild {
    #[serde(default)]
    pub(crate) index: u32,
    #[serde(default)]
    pub(crate) elapsed_ms: u64,
    #[serde(default)]
    pub(crate) planned_size: f64,
    #[serde(default)]
    pub(crate) limit_price: f64,
    #[serde(default)]
    pub(crate) filled_size: f64,
    #[serde(default)]
    pub(crate) avg_price: Option<f64>,
    #[serde(default)]
    pub(crate) fee: f64,
    #[serde(default)]
    pub(crate) oid: Option<u64>,
    #[serde(default)]
    pub(crate) cloid: Option<String>,
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) exchange_summary: String,
}

impl fmt::Debug for AdvancedOrderHistoryChild {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AdvancedOrderHistoryChild")
            .field("index", &self.index)
            .field("elapsed_ms", &"<redacted>")
            .field("planned_size", &"<redacted>")
            .field("limit_price", &"<redacted>")
            .field("filled_size", &"<redacted>")
            .field("has_avg_price", &self.avg_price.is_some())
            .field("fee", &"<redacted>")
            .field("has_oid", &self.oid.is_some())
            .field("has_cloid", &self.cloid.is_some())
            .field("status", &"<redacted>")
            .field("exchange_summary", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct AdvancedOrderHistoryEntry {
    #[serde(default)]
    pub(crate) id: String,
    #[serde(default = "default_history_kind")]
    pub(crate) kind: AdvancedOrderHistoryKind,
    #[serde(default)]
    pub(crate) source_id: u64,
    #[serde(default)]
    pub(crate) account_address: String,
    #[serde(default)]
    pub(crate) coin: String,
    #[serde(default)]
    pub(crate) display_coin: String,
    #[serde(default)]
    pub(crate) is_buy: bool,
    #[serde(default)]
    pub(crate) target_size: f64,
    #[serde(default)]
    pub(crate) filled_size: f64,
    #[serde(default)]
    pub(crate) remaining_size: f64,
    #[serde(default)]
    pub(crate) average_price: Option<f64>,
    #[serde(default)]
    pub(crate) last_working_price: Option<f64>,
    #[serde(default)]
    pub(crate) gross_notional: f64,
    #[serde(default)]
    pub(crate) total_fee: f64,
    #[serde(default)]
    pub(crate) closed_pnl: f64,
    #[serde(default)]
    pub(crate) min_price: Option<f64>,
    #[serde(default)]
    pub(crate) max_price: Option<f64>,
    #[serde(default)]
    pub(crate) reduce_only: bool,
    #[serde(default)]
    pub(crate) randomize: bool,
    #[serde(default)]
    pub(crate) slice_count: u32,
    #[serde(default)]
    pub(crate) slices_sent: u32,
    #[serde(default)]
    pub(crate) reprice_count: u32,
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) summary: String,
    #[serde(default)]
    pub(crate) started_at_ms: u64,
    #[serde(default)]
    pub(crate) completed_at_ms: u64,
    #[serde(default)]
    pub(crate) logs: Vec<AdvancedOrderHistoryLog>,
    #[serde(default)]
    pub(crate) children: Vec<AdvancedOrderHistoryChild>,
}

impl fmt::Debug for AdvancedOrderHistoryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AdvancedOrderHistoryEntry")
            .field("id", &"<redacted>")
            .field("kind", &self.kind)
            .field("source_id", &self.source_id)
            .field("account_address", &"<redacted>")
            .field("coin", &"<redacted>")
            .field("display_coin", &"<redacted>")
            .field("is_buy", &self.is_buy)
            .field("target_size", &"<redacted>")
            .field("filled_size", &"<redacted>")
            .field("remaining_size", &"<redacted>")
            .field("has_average_price", &self.average_price.is_some())
            .field("has_last_working_price", &self.last_working_price.is_some())
            .field("gross_notional", &"<redacted>")
            .field("total_fee", &"<redacted>")
            .field("closed_pnl", &"<redacted>")
            .field("has_min_price", &self.min_price.is_some())
            .field("has_max_price", &self.max_price.is_some())
            .field("reduce_only", &self.reduce_only)
            .field("randomize", &self.randomize)
            .field("slice_count", &"<redacted>")
            .field("slices_sent", &"<redacted>")
            .field("reprice_count", &"<redacted>")
            .field("status", &"<redacted>")
            .field("summary", &"<redacted>")
            .field("started_at_ms", &"<redacted>")
            .field("completed_at_ms", &"<redacted>")
            .field("logs_count", &self.logs.len())
            .field("children_count", &self.children.len())
            .finish()
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct ChaseHistoryFillMetrics {
    pub(crate) filled_size: f64,
    pub(crate) gross_notional: f64,
    pub(crate) total_fee: f64,
    pub(crate) closed_pnl: f64,
}

impl fmt::Debug for ChaseHistoryFillMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChaseHistoryFillMetrics")
            .field("filled_size", &"<redacted>")
            .field("gross_notional", &"<redacted>")
            .field("total_fee", &"<redacted>")
            .field("closed_pnl", &"<redacted>")
            .finish()
    }
}

impl ChaseHistoryFillMetrics {
    pub(crate) fn average_price(self) -> Option<f64> {
        if self.filled_size.is_finite() && self.filled_size > 0.0 && self.gross_notional.is_finite()
        {
            Some(self.gross_notional / self.filled_size)
        } else {
            None
        }
    }
}

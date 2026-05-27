use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct ChaseHistoryFillMetrics {
    pub(crate) filled_size: f64,
    pub(crate) gross_notional: f64,
    pub(crate) total_fee: f64,
    pub(crate) closed_pnl: f64,
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

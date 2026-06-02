use super::identity::FillIdentity;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AggregatedTrade {
    pub id: String,
    pub legacy_note_ids: Vec<String>,
    pub coin: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub max_position: f64,
    pub volume: f64,
    pub fee: f64,
    pub pnl: f64,
    pub status: String,
    pub fill_count: usize,
    pub avg_entry_price: f64,
    pub total_entry_notional: f64,
    pub total_entry_size: f64,
    pub is_long: bool,
    pub basis_complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalAttributedFillRole {
    Increase,
    Reduce,
    FlipClose,
    FlipOpen,
    Settlement,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JournalAttributedFill {
    pub identity: FillIdentity,
    pub time_ms: u64,
    pub price: f64,
    pub raw_size: f64,
    pub attributed_size: f64,
    pub side: String,
    pub role: JournalAttributedFillRole,
    pub fee: f64,
    pub closed_pnl: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct JournalTradeDetails {
    pub trade_id: String,
    pub coin: String,
    pub attributed_fills: Vec<JournalAttributedFill>,
}

#[derive(Debug, Clone, Default)]
pub struct AggregationDiagnostics {
    pub skipped_fill_count: usize,
    pub incomplete_trade_count: usize,
    pub same_timestamp_position_mismatch_count: usize,
}

impl AggregationDiagnostics {
    pub fn warning_message(&self) -> Option<String> {
        let mut parts = Vec::new();

        if self.skipped_fill_count > 0 {
            parts.push(format!(
                "{} fill{} skipped because numeric fields could not be parsed",
                self.skipped_fill_count,
                if self.skipped_fill_count == 1 {
                    ""
                } else {
                    "s"
                }
            ));
        }

        if self.incomplete_trade_count > 0 {
            parts.push(format!(
                "{} trade{} marked partial because opening history is outside the loaded fills",
                self.incomplete_trade_count,
                if self.incomplete_trade_count == 1 {
                    ""
                } else {
                    "s"
                }
            ));
        }

        if self.same_timestamp_position_mismatch_count > 0 {
            parts.push(format!(
                "{} same-timestamp fill{} used API startPosition because local position tracking was discontinuous",
                self.same_timestamp_position_mismatch_count,
                if self.same_timestamp_position_mismatch_count == 1 {
                    ""
                } else {
                    "s"
                }
            ));
        }

        if parts.is_empty() {
            None
        } else {
            Some(format!("Journal data quality: {}.", parts.join("; ")))
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AggregationResult {
    pub trades: Vec<AggregatedTrade>,
    pub trade_details: HashMap<String, JournalTradeDetails>,
    pub diagnostics: AggregationDiagnostics,
}

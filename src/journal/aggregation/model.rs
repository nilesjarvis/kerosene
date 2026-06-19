use super::identity::FillIdentity;
use std::{collections::HashMap, fmt};

#[derive(Clone)]
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

impl fmt::Debug for AggregatedTrade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AggregatedTrade")
            .field("id", &format_args!("<redacted>"))
            .field(
                "legacy_note_ids",
                &format_args!("len={}", self.legacy_note_ids.len()),
            )
            .field("coin", &format_args!("<redacted>"))
            .field("time_range", &format_args!("<redacted>"))
            .field("metrics", &format_args!("<redacted>"))
            .field("status", &self.status)
            .field("fill_count", &self.fill_count)
            .field("basis_complete", &self.basis_complete)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalAttributedFillRole {
    Increase,
    Reduce,
    FlipClose,
    FlipOpen,
    Settlement,
}

#[derive(Clone, PartialEq)]
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

impl fmt::Debug for JournalAttributedFill {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JournalAttributedFill")
            .field("identity", &format_args!("<redacted>"))
            .field("time_ms", &format_args!("<redacted>"))
            .field("metrics", &format_args!("<redacted>"))
            .field("role", &self.role)
            .finish()
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct JournalTradeDetails {
    pub trade_id: String,
    pub coin: String,
    pub attributed_fills: Vec<JournalAttributedFill>,
}

impl fmt::Debug for JournalTradeDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JournalTradeDetails")
            .field("trade_id", &format_args!("<redacted>"))
            .field("coin", &format_args!("<redacted>"))
            .field(
                "attributed_fills",
                &format_args!("len={}", self.attributed_fills.len()),
            )
            .finish()
    }
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

#[derive(Clone, Default)]
pub struct AggregationResult {
    pub trades: Vec<AggregatedTrade>,
    pub trade_details: HashMap<String, JournalTradeDetails>,
    pub diagnostics: AggregationDiagnostics,
}

impl fmt::Debug for AggregationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AggregationResult")
            .field("trades", &format_args!("len={}", self.trades.len()))
            .field(
                "trade_details",
                &format_args!("len={}", self.trade_details.len()),
            )
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AggregatedTrade, AggregationDiagnostics, AggregationResult, FillIdentity,
        JournalAttributedFill, JournalAttributedFillRole, JournalTradeDetails,
    };
    use std::collections::HashMap;

    #[test]
    fn aggregated_trade_debug_redacts_trade_values() {
        let trade = aggregated_trade();

        let rendered = format!("{trade:?}");

        assert!(rendered.contains("id: <redacted>"));
        assert!(rendered.contains("legacy_note_ids: len=1"));
        assert!(rendered.contains("coin: <redacted>"));
        assert!(rendered.contains("metrics: <redacted>"));
        assert!(rendered.contains("fill_count: 2"));
        for secret in [
            "trade-secret-id",
            "SECRETCOIN",
            "note-secret-id",
            "12345.67",
        ] {
            assert!(!rendered.contains(secret), "{secret} leaked in {rendered}");
        }
    }

    #[test]
    fn journal_trade_details_debug_summarizes_attributed_fills() {
        let details = JournalTradeDetails {
            trade_id: "trade-secret-id".to_string(),
            coin: "SECRETCOIN".to_string(),
            attributed_fills: vec![attributed_fill()],
        };

        let rendered_details = format!("{details:?}");
        let rendered_fill = format!("{:?}", details.attributed_fills[0]);

        assert!(rendered_details.contains("trade_id: <redacted>"));
        assert!(rendered_details.contains("coin: <redacted>"));
        assert!(rendered_details.contains("attributed_fills: len=1"));
        assert!(rendered_fill.contains("identity: <redacted>"));
        assert!(rendered_fill.contains("metrics: <redacted>"));
        assert!(rendered_fill.contains("role: Reduce"));
        for secret in [
            "trade-secret-id",
            "SECRETCOIN",
            "fill-secret-hash",
            "fill-secret-price",
            "12345.67",
        ] {
            assert!(
                !rendered_details.contains(secret),
                "{secret} leaked in {rendered_details}"
            );
            assert!(
                !rendered_fill.contains(secret),
                "{secret} leaked in {rendered_fill}"
            );
        }
    }

    #[test]
    fn aggregation_result_debug_summarizes_collections() {
        let result = AggregationResult {
            trades: vec![aggregated_trade()],
            trade_details: HashMap::from([(
                "trade-secret-id".to_string(),
                JournalTradeDetails {
                    trade_id: "trade-secret-id".to_string(),
                    coin: "SECRETCOIN".to_string(),
                    attributed_fills: vec![attributed_fill()],
                },
            )]),
            diagnostics: AggregationDiagnostics {
                skipped_fill_count: 1,
                incomplete_trade_count: 2,
                same_timestamp_position_mismatch_count: 3,
            },
        };

        let rendered = format!("{result:?}");

        assert!(rendered.contains("trades: len=1"));
        assert!(rendered.contains("trade_details: len=1"));
        assert!(rendered.contains("skipped_fill_count: 1"));
        for secret in ["trade-secret-id", "SECRETCOIN", "fill-secret-hash"] {
            assert!(!rendered.contains(secret), "{secret} leaked in {rendered}");
        }
    }

    fn aggregated_trade() -> AggregatedTrade {
        AggregatedTrade {
            id: "trade-secret-id".to_string(),
            legacy_note_ids: vec!["note-secret-id".to_string()],
            coin: "SECRETCOIN".to_string(),
            start_time: 10,
            end_time: Some(20),
            max_position: 12345.67,
            volume: 23456.78,
            fee: 12.34,
            pnl: 56.78,
            status: "Closed".to_string(),
            fill_count: 2,
            avg_entry_price: 100.0,
            total_entry_notional: 1000.0,
            total_entry_size: 10.0,
            is_long: true,
            basis_complete: false,
        }
    }

    fn attributed_fill() -> JournalAttributedFill {
        JournalAttributedFill {
            identity: FillIdentity {
                time: 10,
                tid: 11,
                oid: 12,
                hash: "fill-secret-hash".to_string(),
                coin: "SECRETCOIN".to_string(),
                side: "B".to_string(),
                px: "fill-secret-price".to_string(),
                sz: "1".to_string(),
            },
            time_ms: 10,
            price: 12345.67,
            raw_size: 1.0,
            attributed_size: 1.0,
            side: "B".to_string(),
            role: JournalAttributedFillRole::Reduce,
            fee: 1.23,
            closed_pnl: 4.56,
        }
    }
}

use crate::chart_state::{ChartId, ChartSurfaceId};
use crate::config::QuickTradeActionConfig;

use std::fmt;

// ---------------------------------------------------------------------------
// Quick Trade Order Request
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
pub(crate) struct QuickTradeOrderRequest {
    pub(crate) chart_id: ChartId,
    pub(crate) surface_id: ChartSurfaceId,
    pub(crate) symbol_key: String,
    pub(crate) action_index: usize,
    pub(crate) action: QuickTradeActionConfig,
}

impl fmt::Debug for QuickTradeOrderRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QuickTradeOrderRequest")
            .field("chart_id", &self.chart_id)
            .field("surface_id", &self.surface_id)
            .field("symbol_key", &format_args!("<redacted>"))
            .field("action_index", &self.action_index)
            .field("action", &format_args!("<redacted>"))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{QuickTradeDenomination, QuickTradeSide};

    #[test]
    fn request_debug_redacts_symbol_and_quantity() {
        let request = QuickTradeOrderRequest {
            chart_id: 7,
            surface_id: ChartSurfaceId::Docked(7),
            symbol_key: "SECRETCOIN".to_string(),
            action_index: 2,
            action: QuickTradeActionConfig {
                side: QuickTradeSide::Buy,
                quantity: 98_765.432_1,
                denomination: QuickTradeDenomination::Usd,
            },
        };

        let rendered = format!("{request:?}");
        assert!(rendered.contains("symbol_key: <redacted>"));
        assert!(rendered.contains("action: <redacted>"));
        assert!(!rendered.contains("SECRETCOIN"));
        assert!(!rendered.contains("98765"));
    }
}

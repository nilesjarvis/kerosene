use crate::account;
use crate::app_state::TradingTerminal;

// ---------------------------------------------------------------------------
// Account Value
// ---------------------------------------------------------------------------

impl TradingTerminal {
    /// The Total PnL % denominator must be the same headline account value
    /// the connected summary header shows, so the two can never diverge
    /// (shared-balance accounts dedupe the mirrored perp/spot USDC pool
    /// instead of double counting it).
    pub(super) fn position_summary_account_value(
        &self,
        data: &account::AccountData,
    ) -> Option<f64> {
        self.account_summary_total_value(data)
    }
}

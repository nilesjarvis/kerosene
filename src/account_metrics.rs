use crate::app_state::TradingTerminal;

use crate::account;

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(crate) fn parse_liquidation_px(ap: &account::AssetPosition) -> Option<f64> {
        ap.position
            .liquidation_px
            .as_deref()
            .or(ap.liquidation_px.as_deref())
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|v| v.is_finite() && *v > 0.0)
    }

    pub(crate) fn position_funding_pnl(cum_funding: Option<&account::CumFunding>) -> Option<f64> {
        cum_funding
            .and_then(|cf| cf.since_open.trim().parse::<f64>().ok())
            .filter(|value| value.is_finite())
            .map(|raw_payment| -raw_payment)
    }
}

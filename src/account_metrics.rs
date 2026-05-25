use crate::{
    account,
    app_state::TradingTerminal,
    helpers::{parse_finite_number, parse_positive_finite_number},
};

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(crate) fn parse_liquidation_px(ap: &account::AssetPosition) -> Option<f64> {
        ap.position
            .liquidation_px
            .as_deref()
            .or(ap.liquidation_px.as_deref())
            .and_then(parse_positive_finite_number)
    }

    pub(crate) fn position_funding_pnl(cum_funding: Option<&account::CumFunding>) -> Option<f64> {
        cum_funding
            .and_then(|cf| parse_finite_number(&cf.since_open))
            .map(|raw_payment| -raw_payment)
    }
}

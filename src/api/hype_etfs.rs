use crate::hype_etf_state::HypeEtfData;

mod bhyp;
mod http;
mod numbers;
mod thyp;

use bhyp::fetch_bhyp;
use thyp::fetch_thyp;

#[cfg(test)]
use crate::hype_etf_state::HypeEtfTicker;
#[cfg(test)]
use bhyp::{BhypResponse, bhyp_fund_from_response};
#[cfg(test)]
use thyp::{ThypResponse, thyp_daily_flows, thyp_fund_from_response};

// ---------------------------------------------------------------------------
// HYPE ETF API
// ---------------------------------------------------------------------------

pub(crate) async fn fetch_hype_etfs() -> Result<HypeEtfData, String> {
    let (thyp, bhyp) = futures::future::join(fetch_thyp(), fetch_bhyp()).await;
    let mut funds = Vec::new();
    let mut warnings = Vec::new();

    match thyp {
        Ok(fund) => funds.push(fund),
        Err(error) => warnings.push(error),
    }
    match bhyp {
        Ok(fund) => funds.push(fund),
        Err(error) => warnings.push(error),
    }

    if funds.is_empty() {
        return Err(warnings.join("; "));
    }

    Ok(HypeEtfData { funds, warnings })
}

#[cfg(test)]
mod tests;

use crate::hype_etf_state::HypeEtfData;

use std::time::Duration;

mod bhyp;
mod farside;
mod http;
mod numbers;
mod thyp;

use bhyp::fetch_bhyp;
use farside::fetch_farside_bhyp_flows;
use thyp::fetch_thyp;

#[cfg(test)]
use crate::hype_etf_state::HypeEtfTicker;
#[cfg(test)]
use bhyp::{BhypResponse, bhyp_fund_from_response};
#[cfg(test)]
use thyp::{ThypResponse, thyp_daily_flows, thyp_fund_from_response};

const FARSIDE_BHYP_FLOW_TIMEOUT: Duration = Duration::from_secs(5);

// ---------------------------------------------------------------------------
// HYPE ETF API
// ---------------------------------------------------------------------------

pub(crate) async fn fetch_hype_etfs() -> Result<HypeEtfData, String> {
    let bhyp_flow_task = async {
        match tokio::time::timeout(FARSIDE_BHYP_FLOW_TIMEOUT, fetch_farside_bhyp_flows()).await {
            Ok(result) => result,
            Err(_) => Err(format!(
                "request timed out after {}s",
                FARSIDE_BHYP_FLOW_TIMEOUT.as_secs()
            )),
        }
    };

    let ((thyp, mut bhyp), bhyp_flows) = futures::future::join(
        futures::future::join(fetch_thyp(), fetch_bhyp()),
        bhyp_flow_task,
    )
    .await;

    let mut funds = Vec::new();
    let mut warnings = Vec::new();

    match thyp {
        Ok(fund) => funds.push(fund),
        Err(error) => warnings.push(error),
    }

    // Merge Farside BHYP flow data into the BHYP fund if available.
    match (bhyp.as_mut(), bhyp_flows) {
        (Ok(fund), Ok(flows)) => {
            fund.daily_flows = flows;
        }
        (_, Err(farside_error)) => {
            warnings.push(format!("BHYP flow history unavailable: {farside_error}"));
        }
        _ => {}
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

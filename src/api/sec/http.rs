use crate::api::CLIENT;
use crate::network_activity::HttpRequestExt as _;
use reqwest::{Response, header::USER_AGENT};
use serde::Deserialize;

pub(super) const SEC_TICKER_MAP_URL: &str = "https://www.sec.gov/files/company_tickers.json";
pub(super) const SEC_SUBMISSIONS_BASE_URL: &str = "https://data.sec.gov/submissions";
pub(super) const SEC_COMPANY_FACTS_BASE_URL: &str = "https://data.sec.gov/api/xbrl/companyfacts";
pub(super) const SEC_ARCHIVES_BASE_URL: &str = "https://www.sec.gov/Archives/edgar/data";

const DEFAULT_SEC_USER_AGENT: &str = concat!(
    "Kerosene/",
    env!("CARGO_PKG_VERSION"),
    " sec-edgar@kerosene.local"
);

pub(super) async fn sec_get_json<T>(url: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    sec_get_response(url)
        .await?
        .json()
        .await
        .map_err(|e| format!("SEC response parse failed: {e}"))
}

pub(super) async fn sec_get_text(url: &str) -> Result<String, String> {
    sec_get_response(url)
        .await?
        .text()
        .await
        .map_err(|e| format!("SEC text response parse failed: {e}"))
}

async fn sec_get_response(url: &str) -> Result<Response, String> {
    let response = CLIENT
        .get(url)
        .header(USER_AGENT, sec_user_agent())
        .send_observed()
        .await
        .map_err(|e| format!("SEC request failed: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "SEC request to {} returned {status}",
            sec_endpoint_label(url)
        ));
    }

    Ok(response)
}

fn sec_user_agent() -> String {
    std::env::var("KEROSENE_SEC_USER_AGENT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_SEC_USER_AGENT.to_string())
}

fn sec_endpoint_label(url: &str) -> &'static str {
    if url == SEC_TICKER_MAP_URL {
        "ticker map"
    } else if url.starts_with(SEC_SUBMISSIONS_BASE_URL) {
        "company submissions"
    } else if url.starts_with(SEC_COMPANY_FACTS_BASE_URL) {
        "company facts"
    } else if url.starts_with(SEC_ARCHIVES_BASE_URL) {
        "filing archive"
    } else {
        "EDGAR API"
    }
}

#[cfg(test)]
mod tests;

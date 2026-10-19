//! Public Papertrade market discovery. Account access and execution are deliberately
//! separate from this catalog and from Hyperliquid's exchange-symbol registry.

pub(crate) mod markets;

pub(crate) use markets::PapertradeMarkets;

use crate::network_activity::HttpRequestExt as _;

const MARKETS_URL: &str = "https://exchange.papertrade.xyz/state/trading";
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

pub(crate) async fn fetch_markets() -> Result<PapertradeMarkets, String> {
    fetch_markets_request(crate::api::CLIENT.get(MARKETS_URL)).await
}

async fn fetch_markets_request(
    request: reqwest::RequestBuilder,
) -> Result<PapertradeMarkets, String> {
    let mut response = request
        .send_observed()
        .await
        .map_err(|_| "Could not reach Papertrade. Try again.".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Papertrade markets unavailable (HTTP {}).",
            response.status().as_u16()
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Could not read Papertrade markets. Try again.".to_string())?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err("Papertrade market response is too large.".to_string());
        }
        bytes.extend_from_slice(&chunk);
    }
    markets::parse_markets(&bytes)
}

#[cfg(test)]
mod tests;

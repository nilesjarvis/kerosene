use super::{API_URL, CLIENT};
use serde::de::DeserializeOwned;
use serde_json::Value;

mod model;
mod outcomes;
mod perps;
mod spot;

pub use model::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use outcomes::{OutcomeMetaResponse, append_outcome_symbols};
use perps::append_perp_symbols;
use spot::append_spot_symbols;

/// Result of a symbols fetch. Spot and outcome metadata failures are partial:
/// perp symbols still load, but the failed source's symbols are absent and the
/// caller must keep any previously loaded symbols of that market type.
#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeSymbolsPayload {
    pub symbols: Vec<ExchangeSymbol>,
    pub spot_meta_failed: bool,
    pub outcome_meta_failed: bool,
}

/// Fetch all tradeable symbols (perps + spot + outcomes) by combining
/// allPerpMetas, perpConciseAnnotations, perpDexs, spotMeta, and outcomeMeta.
pub async fn fetch_exchange_symbols() -> Result<ExchangeSymbolsPayload, String> {
    let client = CLIENT.clone();

    let (metas_raw, annotations_raw, dexs_raw) = futures::try_join!(
        post_info_value(client.clone(), "allPerpMetas"),
        post_info_value(client.clone(), "perpConciseAnnotations"),
        post_info_value(client.clone(), "perpDexs"),
    )?;

    let mut symbols = Vec::new();
    append_perp_symbols(&mut symbols, &metas_raw, &annotations_raw, &dexs_raw)?;

    let (spot_meta_result, outcome_meta_result) = futures::join!(
        post_info_value(client.clone(), "spotMeta"),
        post_info_typed::<OutcomeMetaResponse>(client, "outcomeMeta"),
    );

    let spot_meta_failed = match spot_meta_result {
        Ok(spot_meta) => {
            append_spot_symbols(&mut symbols, &spot_meta);
            false
        }
        Err(_) => true,
    };

    let outcome_meta_failed = match outcome_meta_result {
        Ok(outcome_meta) => {
            append_outcome_symbols(&mut symbols, outcome_meta);
            false
        }
        Err(_) => true,
    };

    symbols.sort_by(|a, b| a.ticker.cmp(&b.ticker));

    Ok(ExchangeSymbolsPayload {
        symbols,
        spot_meta_failed,
        outcome_meta_failed,
    })
}

fn info_request_payload(request_type: &'static str) -> serde_json::Value {
    serde_json::json!({ "type": request_type })
}

async fn post_info_value(
    client: reqwest::Client,
    request_type: &'static str,
) -> Result<Value, String> {
    post_info_typed(client, request_type).await
}

async fn post_info_typed<T: DeserializeOwned>(
    client: reqwest::Client,
    request_type: &'static str,
) -> Result<T, String> {
    let response = client
        .post(API_URL)
        .json(&info_request_payload(request_type))
        .send()
        .await
        .map_err(|e| format!("{request_type} request failed: {e}"))?;

    response
        .json()
        .await
        .map_err(|e| format!("{request_type} parse failed: {e}"))
}

#[cfg(test)]
mod tests;

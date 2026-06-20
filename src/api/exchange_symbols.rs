use super::{API_URL, CLIENT};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::fmt;

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
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct ExchangeSymbolsPayload {
    pub symbols: Vec<ExchangeSymbol>,
    pub spot_meta_failed: bool,
    pub outcome_meta_failed: bool,
}

impl fmt::Debug for ExchangeSymbolsPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut perp_count = 0;
        let mut spot_count = 0;
        let mut outcome_count = 0;

        for symbol in &self.symbols {
            match symbol.market_type {
                MarketType::Perp => perp_count += 1,
                MarketType::Spot => spot_count += 1,
                MarketType::Outcome => outcome_count += 1,
            }
        }

        f.debug_struct("ExchangeSymbolsPayload")
            .field("symbols_len", &self.symbols.len())
            .field("perp_count", &perp_count)
            .field("spot_count", &spot_count)
            .field("outcome_count", &outcome_count)
            .field("spot_meta_failed", &self.spot_meta_failed)
            .field("outcome_meta_failed", &self.outcome_meta_failed)
            .finish()
    }
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

    let payload = ExchangeSymbolsPayload {
        symbols,
        spot_meta_failed,
        outcome_meta_failed,
    };
    if !payload.spot_meta_failed && !payload.outcome_meta_failed {
        let _ = crate::api_cache::save_exchange_symbols(&payload);
    }

    Ok(payload)
}

pub async fn fetch_exchange_symbols_cached() -> Result<ExchangeSymbolsPayload, String> {
    let now_ms = crate::app_time::now_ms();
    if let Ok(Some(payload)) = crate::api_cache::load_fresh_exchange_symbols(now_ms) {
        return Ok(payload);
    }

    fetch_exchange_symbols().await
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

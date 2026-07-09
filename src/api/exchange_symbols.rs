use super::{API_URL, CLIENT};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::fmt;

mod model;
mod outcomes;
mod perps;
mod spot;

pub use model::{ExchangeSymbol, MarketType, OutcomeSymbolInfo, spot_symbol_for_indexed_key};
use outcomes::{OutcomeMetaResponse, append_outcome_symbols};
use perps::append_perp_symbols;
use spot::append_spot_symbols;

/// Result of a symbols fetch. Spot and outcome metadata failures are partial:
/// the other sources still load, but the failed source's symbols are absent and
/// the caller must keep any previously loaded symbols of that market type.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct ExchangeSymbolsPayload {
    pub symbols: Vec<ExchangeSymbol>,
    /// Runtime-only provenance. Cached metadata is useful for rendering, but
    /// must never authorize spot trading until a live strict refresh succeeds.
    #[serde(skip)]
    pub loaded_from_cache: bool,
    #[serde(default)]
    pub perp_meta_failed: bool,
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
            .field("loaded_from_cache", &self.loaded_from_cache)
            .field("perp_meta_failed", &self.perp_meta_failed)
            .field("spot_meta_failed", &self.spot_meta_failed)
            .field("outcome_meta_failed", &self.outcome_meta_failed)
            .finish()
    }
}

impl ExchangeSymbolsPayload {
    /// Whether this payload is safe to persist as the last-known complete
    /// exchange universe. Legacy spot entries without their quote token are
    /// deliberately rejected so percentage buys never silently assume USDC.
    pub(crate) fn is_cacheable(&self) -> bool {
        !self.loaded_from_cache
            && !self.perp_meta_failed
            && !self.spot_meta_failed
            && !self.outcome_meta_failed
            && self
                .symbols
                .iter()
                .any(|symbol| symbol.market_type == MarketType::Spot)
            && self.symbols.iter().all(|symbol| {
                symbol.market_type != MarketType::Spot || symbol.collateral_token.is_some()
            })
    }
}

/// Fetch all tradeable symbols (perps + spot + outcomes) by combining
/// allPerpMetas, perpConciseAnnotations, perpDexs, spotMeta, and outcomeMeta.
pub async fn fetch_exchange_symbols() -> Result<ExchangeSymbolsPayload, String> {
    let client = CLIENT.clone();
    let perp_client = client.clone();
    let spot_client = client.clone();
    let outcome_client = client;

    // Fetch each market family independently. Spot discovery and quote-token
    // safety must not depend on unrelated perp metadata being available.
    let (perp_result, spot_result, outcome_result) = futures::join!(
        async move {
            let (metas_raw, annotations_raw, dexs_raw) = futures::try_join!(
                post_info_value(perp_client.clone(), "allPerpMetas"),
                post_info_value(perp_client.clone(), "perpConciseAnnotations"),
                post_info_value(perp_client, "perpDexs"),
            )?;
            let mut symbols = Vec::new();
            append_perp_symbols(&mut symbols, &metas_raw, &annotations_raw, &dexs_raw)?;
            Ok::<_, String>(symbols)
        },
        async move {
            let spot_meta = post_info_value(spot_client, "spotMeta").await?;
            let mut symbols = Vec::new();
            append_spot_symbols(&mut symbols, &spot_meta)?;
            Ok::<_, String>(symbols)
        },
        async move {
            let outcome_meta =
                post_info_typed::<OutcomeMetaResponse>(outcome_client, "outcomeMeta").await?;
            let mut symbols = Vec::new();
            append_outcome_symbols(&mut symbols, outcome_meta);
            Ok::<_, String>(symbols)
        },
    );

    let payload = payload_from_source_results(perp_result, spot_result, outcome_result);
    if payload.is_cacheable() {
        let _ = crate::api_cache::save_exchange_symbols(&payload);
    }

    Ok(payload)
}

fn payload_from_source_results(
    perp_result: Result<Vec<ExchangeSymbol>, String>,
    spot_result: Result<Vec<ExchangeSymbol>, String>,
    outcome_result: Result<Vec<ExchangeSymbol>, String>,
) -> ExchangeSymbolsPayload {
    let perp_meta_failed = perp_result.is_err();
    let spot_meta_failed = spot_result.is_err();
    let outcome_meta_failed = outcome_result.is_err();
    let mut symbols = Vec::new();
    symbols.extend(perp_result.unwrap_or_default());
    symbols.extend(spot_result.unwrap_or_default());
    symbols.extend(outcome_result.unwrap_or_default());

    symbols.sort_by(|a, b| a.ticker.cmp(&b.ticker));

    ExchangeSymbolsPayload {
        symbols,
        loaded_from_cache: false,
        perp_meta_failed,
        spot_meta_failed,
        outcome_meta_failed,
    }
}

pub async fn fetch_exchange_symbols_cached() -> Result<ExchangeSymbolsPayload, String> {
    let now_ms = crate::app_time::now_ms();
    if let Ok(Some(payload)) = crate::api_cache::load_fresh_exchange_symbols(now_ms) {
        return Ok(mark_payload_loaded_from_cache(payload));
    }

    fetch_exchange_symbols().await
}

fn mark_payload_loaded_from_cache(mut payload: ExchangeSymbolsPayload) -> ExchangeSymbolsPayload {
    payload.loaded_from_cache = true;
    payload
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
        .error_for_status()
        .map_err(|e| format!("{request_type} HTTP error: {e}"))?
        .json()
        .await
        .map_err(|e| format!("{request_type} parse failed: {e}"))
}

#[cfg(test)]
mod tests;

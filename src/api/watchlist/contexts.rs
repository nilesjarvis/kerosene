use super::model::{WatchlistContext, WatchlistContextsResponse};
use super::parsing::{
    append_perp_contexts_for_symbols, append_spot_contexts_for_symbols, insert_empty_context,
};
use crate::api::{API_URL, CLIENT};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

enum ContextFamily {
    Perp {
        label: String,
        dex: Option<String>,
        requested_symbols: HashSet<String>,
    },
    Spot {
        requested_symbols: HashSet<String>,
    },
}

type ContextFamilyResult = (String, Result<HashMap<String, WatchlistContext>, String>);

impl ContextFamily {
    fn label(&self) -> String {
        match self {
            Self::Perp { label, .. } => label.clone(),
            Self::Spot { .. } => "spot".to_string(),
        }
    }
}

pub async fn fetch_watchlist_contexts(
    symbols: Vec<String>,
) -> Result<WatchlistContextsResponse, String> {
    let now_ms = crate::app_time::now_ms();
    if let Ok(Some(cached)) = crate::api_cache::load_fresh_watchlist_contexts(&symbols, now_ms) {
        return Ok(WatchlistContextsResponse::complete(cached));
    }

    let mut map = HashMap::new();
    let mut main_symbols = HashSet::new();
    let mut dex_symbols: BTreeMap<String, HashSet<String>> = BTreeMap::new();
    let mut spot_symbols = HashSet::new();

    for symbol in &symbols {
        if symbol.starts_with('#') {
            insert_empty_context(&mut map, symbol);
        } else if symbol.starts_with('@') || symbol.contains('/') {
            spot_symbols.insert(symbol.clone());
        } else if let Some((dex, _)) = symbol.split_once(':') {
            dex_symbols
                .entry(dex.to_string())
                .or_default()
                .insert(symbol.clone());
        } else {
            main_symbols.insert(symbol.clone());
        }
    }

    let mut families = Vec::new();
    if !main_symbols.is_empty() {
        families.push(ContextFamily::Perp {
            label: "main perps".to_string(),
            dex: None,
            requested_symbols: main_symbols,
        });
    }
    families.extend(
        dex_symbols
            .into_iter()
            .map(|(dex, requested_symbols)| ContextFamily::Perp {
                label: format!("HIP-3 dex {dex}"),
                dex: Some(dex),
                requested_symbols,
            }),
    );
    if !spot_symbols.is_empty() {
        families.push(ContextFamily::Spot {
            requested_symbols: spot_symbols,
        });
    }

    let client = CLIENT.clone();
    let results = futures::future::join_all(
        families
            .into_iter()
            .map(|family| fetch_context_family(client.clone(), family)),
    )
    .await;
    let response = merge_context_family_results(map, results)?;
    let _ = crate::api_cache::save_watchlist_contexts(&response.contexts);
    Ok(response)
}

async fn fetch_context_family(
    client: reqwest::Client,
    family: ContextFamily,
) -> ContextFamilyResult {
    let label = family.label();
    let result = async {
        let body = match &family {
            ContextFamily::Perp { dex: Some(dex), .. } => serde_json::json!({
                "type": "metaAndAssetCtxs",
                "dex": dex
            }),
            ContextFamily::Perp { dex: None, .. } => {
                serde_json::json!({ "type": "metaAndAssetCtxs" })
            }
            ContextFamily::Spot { .. } => {
                serde_json::json!({ "type": "spotMetaAndAssetCtxs" })
            }
        };
        let response: Value = client
            .post(API_URL)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?
            .error_for_status()
            .map_err(|e| format!("HTTP error: {e}"))?
            .json()
            .await
            .map_err(|e| format!("parse failed: {e}"))?;

        let mut contexts = HashMap::new();
        match family {
            ContextFamily::Perp {
                dex,
                requested_symbols,
                ..
            } => {
                append_perp_contexts_for_symbols(
                    response,
                    dex.as_deref(),
                    &requested_symbols,
                    &mut contexts,
                )
                .map_err(|e| format!("payload invalid: {e}"))?;
                contexts.retain(|symbol, _| requested_symbols.contains(symbol));
            }
            ContextFamily::Spot { requested_symbols } => {
                append_spot_contexts_for_symbols(response, &requested_symbols, &mut contexts)
                    .map_err(|e| format!("payload invalid: {e}"))?;
                contexts.retain(|symbol, _| requested_symbols.contains(symbol));
            }
        }
        Ok(contexts)
    }
    .await;
    (label, result)
}

fn merge_context_family_results(
    mut contexts: HashMap<String, WatchlistContext>,
    results: Vec<ContextFamilyResult>,
) -> Result<WatchlistContextsResponse, String> {
    if results.is_empty() {
        return Ok(WatchlistContextsResponse::complete(contexts));
    }

    let mut success_count = usize::from(!contexts.is_empty());
    let mut partial_errors = Vec::new();
    for (label, result) in results {
        match result {
            Ok(family_contexts) => {
                success_count += 1;
                contexts.extend(family_contexts);
            }
            Err(error) => partial_errors.push(format!("{label}: {error}")),
        }
    }

    if success_count == 0 {
        return Err(format!(
            "all requested context families failed: {}",
            partial_errors.join("; ")
        ));
    }

    Ok(WatchlistContextsResponse {
        contexts,
        partial_errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(day_vlm: f64) -> WatchlistContext {
        WatchlistContext {
            funding: None,
            prev_day_px: None,
            day_vlm: Some(day_vlm),
        }
    }

    #[test]
    fn mixed_watchlist_keeps_spot_data_when_unrelated_perp_family_fails() {
        let response = merge_context_family_results(
            HashMap::new(),
            vec![
                ("main perps".to_string(), Err("HTTP 503".to_string())),
                (
                    "spot".to_string(),
                    Ok(HashMap::from([("@107".to_string(), context(42.0))])),
                ),
            ],
        )
        .expect("healthy spot family must survive a perp failure");

        assert_eq!(
            response.contexts.get("@107").and_then(|ctx| ctx.day_vlm),
            Some(42.0)
        );
        assert_eq!(response.partial_errors, vec!["main perps: HTTP 503"]);
    }

    #[test]
    fn every_requested_family_failure_is_a_hard_error() {
        let error = merge_context_family_results(
            HashMap::new(),
            vec![
                ("main perps".to_string(), Err("HTTP 503".to_string())),
                ("spot".to_string(), Err("HTTP 429".to_string())),
            ],
        )
        .expect_err("no usable family data");

        assert!(error.contains("main perps: HTTP 503"));
        assert!(error.contains("spot: HTTP 429"));
    }

    #[test]
    fn local_outcome_context_survives_remote_family_failure() {
        let response = merge_context_family_results(
            HashMap::from([("#1".to_string(), context(0.0))]),
            vec![("spot".to_string(), Err("HTTP 503".to_string()))],
        )
        .expect("local outcome context is usable partial data");

        assert!(response.contexts.contains_key("#1"));
        assert_eq!(response.partial_errors, vec!["spot: HTTP 503"]);
    }
}

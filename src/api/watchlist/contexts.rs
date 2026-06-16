use super::model::WatchlistContext;
use super::parsing::{append_perp_contexts, append_spot_contexts, insert_empty_context};
use crate::api::{API_URL, CLIENT};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

pub async fn fetch_watchlist_contexts(
    symbols: Vec<String>,
) -> Result<HashMap<String, WatchlistContext>, String> {
    let client = CLIENT.clone();
    let mut map = HashMap::new();
    let mut needed_dexes = BTreeSet::new();
    let mut needs_main = false;
    let mut needs_spot = false;

    for symbol in &symbols {
        if symbol.starts_with('#') {
            insert_empty_context(&mut map, symbol);
        } else if symbol.starts_with('@') {
            needs_spot = true;
        } else if let Some((dex, _)) = symbol.split_once(':') {
            needed_dexes.insert(dex.to_string());
        } else {
            needs_main = true;
        }
    }

    if needs_main {
        let resp: Value = client
            .post(API_URL)
            .json(&serde_json::json!({"type": "metaAndAssetCtxs"}))
            .send()
            .await
            .map_err(|e| format!("metaAndAssetCtxs request failed: {e}"))?
            .error_for_status()
            .map_err(|e| format!("metaAndAssetCtxs HTTP error: {e}"))?
            .json()
            .await
            .map_err(|e| format!("metaAndAssetCtxs parse failed: {e}"))?;

        append_perp_contexts(resp, None, &mut map)
            .map_err(|e| format!("metaAndAssetCtxs payload invalid: {e}"))?;
    }

    for dex in needed_dexes {
        let dex_val: Value = client
            .post(API_URL)
            .json(&serde_json::json!({
                "type": "metaAndAssetCtxs",
                "dex": dex.as_str()
            }))
            .send()
            .await
            .map_err(|e| format!("metaAndAssetCtxs request failed for HIP-3 dex {dex}: {e}"))?
            .error_for_status()
            .map_err(|e| format!("metaAndAssetCtxs HTTP error for HIP-3 dex {dex}: {e}"))?
            .json()
            .await
            .map_err(|e| format!("metaAndAssetCtxs parse failed for HIP-3 dex {dex}: {e}"))?;

        append_perp_contexts(dex_val, Some(&dex), &mut map)
            .map_err(|e| format!("metaAndAssetCtxs payload invalid for HIP-3 dex {dex}: {e}"))?;
    }

    if !needs_spot {
        return Ok(map);
    }

    let spot_resp: Value = client
        .post(API_URL)
        .json(&serde_json::json!({"type": "spotMetaAndAssetCtxs"}))
        .send()
        .await
        .map_err(|e| format!("spotMetaAndAssetCtxs request failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("spotMetaAndAssetCtxs HTTP error: {e}"))?
        .json()
        .await
        .map_err(|e| format!("spotMetaAndAssetCtxs parse failed: {e}"))?;

    append_spot_contexts(spot_resp, &mut map)
        .map_err(|e| format!("spotMetaAndAssetCtxs payload invalid: {e}"))?;

    Ok(map)
}

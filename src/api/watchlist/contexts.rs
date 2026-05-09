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
            .json()
            .await
            .map_err(|e| format!("metaAndAssetCtxs parse failed: {e}"))?;

        append_perp_contexts(resp, None, &mut map);
    }

    for dex in needed_dexes {
        let dex_resp = client
            .post(API_URL)
            .json(&serde_json::json!({"type": "metaAndAssetCtxs", "dex": dex}))
            .send()
            .await;

        let dex_val = if let Ok(resp) = dex_resp {
            resp.json::<Value>().await.unwrap_or(Value::Null)
        } else {
            Value::Null
        };

        append_perp_contexts(dex_val, Some(&dex), &mut map);
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
        .json()
        .await
        .map_err(|e| format!("spotMetaAndAssetCtxs parse failed: {e}"))?;

    append_spot_contexts(spot_resp, &mut map);

    Ok(map)
}

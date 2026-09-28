use crate::account::AssetContext;
use serde::Deserialize;
use serde_json::Value;
mod spot;

use spot::fetch_spot_chart_asset_context;
pub(crate) use spot::fetch_spot_chart_asset_contexts;

// ---------------------------------------------------------------------------
// Chart Asset Context REST Fallback
// ---------------------------------------------------------------------------
//
// The chart header's 24h-volume and open-interest metrics are populated from a
// chart's `asset_ctx`, which normally arrives over the `activeAssetCtx`
// WebSocket stream. When that stream does not deliver context for a symbol
// (notably HIP-3 builder-deployed perps, spot `@` symbols, or reconnect gaps),
// the header silently blanks because — unlike candles — there was no REST
// fallback. This module provides one, reusing the same `metaAndAssetCtxs` and
// `spotMetaAndAssetCtxs` requests the watchlist/screener already rely on.

/// Fetch the live [`AssetContext`] for a single chart symbol via REST metadata
/// endpoints.
///
/// For a HIP-3 `dex:coin` symbol the `dex` is split out and sent as the
/// `metaAndAssetCtxs` `dex` parameter (the main perp dex omits it). Spot
/// symbols are looked up in `spotMetaAndAssetCtxs`. Returns `Ok(None)` for
/// symbols that have no asset context here (composite `#`, or a coin absent
/// from the universe).
pub async fn fetch_chart_asset_context(symbol: String) -> Result<Option<AssetContext>, String> {
    if symbol.is_empty() || symbol.starts_with('#') {
        return Ok(None);
    }
    // Spot pairs are keyed "@{index}", except pairs the API names directly
    // ("PURR/USDC"), whose keys carry the "{base}/{quote}" slash.
    if symbol.starts_with('@') || symbol.contains('/') {
        return fetch_spot_chart_asset_context(symbol).await;
    }

    let dex = symbol.split_once(':').map(|(dex, _)| dex.to_string());

    let mut body = serde_json::json!({ "type": "metaAndAssetCtxs" });
    if let Some(dex) = dex.as_deref() {
        body["dex"] = Value::String(dex.to_string());
    }

    let resp = super::shared_reads::public_info(body).await?;

    Ok(parse_chart_asset_context(&resp, &symbol, dex.as_deref()))
}

/// Locate `symbol` within a `metaAndAssetCtxs` `[meta, contexts]` response and
/// deserialize its parallel context object into an [`AssetContext`].
///
/// Builder-deployed perps may be named with either the full `dex:coin` form or
/// the bare coin within the per-dex universe; both are matched against the
/// canonical `dex:coin` key (mirroring `watchlist::parsing::append_perp_contexts`).
pub(crate) fn parse_chart_asset_context(
    resp: &Value,
    symbol: &str,
    dex: Option<&str>,
) -> Option<AssetContext> {
    let arr = resp.as_array()?;
    if arr.len() != 2 {
        return None;
    }
    let universe = arr[0].as_object()?.get("universe")?.as_array()?;
    let ctxs = arr[1].as_array()?;

    for (i, coin_meta) in universe.iter().enumerate() {
        let Some(name) = coin_meta.get("name").and_then(Value::as_str) else {
            continue;
        };
        let canonical_key = if name.contains(':') {
            name.to_string()
        } else if let Some(dex) = dex {
            format!("{dex}:{name}")
        } else {
            name.to_string()
        };
        if canonical_key != symbol {
            continue;
        }
        let ctx_val = ctxs.get(i)?;
        return AssetContext::deserialize(ctx_val).ok();
    }

    None
}

#[cfg(test)]
mod tests;

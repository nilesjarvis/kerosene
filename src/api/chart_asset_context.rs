use super::{API_URL, CLIENT};
use crate::account::AssetContext;
use serde_json::Value;
use std::collections::HashMap;

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

    let resp: Value = CLIENT
        .clone()
        .post(API_URL)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("metaAndAssetCtxs request failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("metaAndAssetCtxs HTTP error: {e}"))?
        .json()
        .await
        .map_err(|e| format!("metaAndAssetCtxs parse failed: {e}"))?;

    Ok(parse_chart_asset_context(&resp, &symbol, dex.as_deref()))
}

async fn fetch_spot_chart_asset_context(symbol: String) -> Result<Option<AssetContext>, String> {
    let mut contexts = fetch_spot_chart_asset_contexts(vec![symbol.clone()]).await?;
    Ok(contexts
        .drain(..)
        .find_map(|(context_symbol, context)| (context_symbol == symbol).then_some(context)))
}

/// Fetch spot contexts for any number of charts with one
/// `spotMetaAndAssetCtxs` request. That endpoint always returns the complete
/// spot universe, so issuing one request per chart wastes the shared REST rate
/// limit and turns transient failures into a retry storm.
pub(crate) async fn fetch_spot_chart_asset_contexts(
    symbols: Vec<String>,
) -> Result<Vec<(String, AssetContext)>, String> {
    if symbols.is_empty() {
        return Ok(Vec::new());
    }

    let response = CLIENT
        .clone()
        .post(API_URL)
        .json(&serde_json::json!({ "type": "spotMetaAndAssetCtxs" }))
        .send()
        .await
        .map_err(|e| format!("spotMetaAndAssetCtxs request failed: {e}"))?;
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry_after = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .map(|value| format!("; retry after {value}"))
            .unwrap_or_default();
        return Err(format!(
            "spotMetaAndAssetCtxs rate limited (HTTP 429){retry_after}"
        ));
    }
    let resp: Value = response
        .error_for_status()
        .map_err(|e| format!("spotMetaAndAssetCtxs HTTP error: {e}"))?
        .json()
        .await
        .map_err(|e| format!("spotMetaAndAssetCtxs parse failed: {e}"))?;
    validate_spot_chart_asset_context_response(&resp)?;

    let mut seen = std::collections::HashSet::new();
    let contexts = symbols
        .into_iter()
        .filter(|symbol| seen.insert(symbol.clone()))
        .filter_map(|symbol| {
            parse_spot_chart_asset_context(&resp, &symbol).map(|context| (symbol, context))
        })
        .collect();
    Ok(contexts)
}

fn validate_spot_chart_asset_context_response(resp: &Value) -> Result<(), String> {
    let response_parts = resp
        .as_array()
        .filter(|parts| parts.len() == 2)
        .ok_or_else(|| {
            "spotMetaAndAssetCtxs schema invalid: expected [meta, contexts]".to_string()
        })?;
    let universe = response_parts[0]
        .get("universe")
        .and_then(Value::as_array)
        .ok_or_else(|| "spotMetaAndAssetCtxs schema invalid: missing meta universe".to_string())?;
    let response_contexts = response_parts[1].as_array().ok_or_else(|| {
        "spotMetaAndAssetCtxs schema invalid: contexts must be an array".to_string()
    })?;
    if universe.is_empty() || response_contexts.is_empty() {
        return Err("spotMetaAndAssetCtxs schema invalid: empty universe or contexts".to_string());
    }
    Ok(())
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
        return serde_json::from_value::<AssetContext>(ctx_val.clone()).ok();
    }

    None
}

/// Locate a spot symbol — the "@{index}" form or an API-named pair like
/// "PURR/USDC" — within a `spotMetaAndAssetCtxs` `[meta, contexts]` response
/// and deserialize its context object.
pub(crate) fn parse_spot_chart_asset_context(resp: &Value, symbol: &str) -> Option<AssetContext> {
    let arr = resp.as_array()?;
    if arr.len() != 2 {
        return None;
    }
    let universe = arr[0].as_object()?.get("universe")?.as_array()?;
    let ctxs = arr[1].as_array()?;
    let ctxs_by_coin = spot_contexts_by_coin(ctxs);
    let allow_unkeyed_fallback = ctxs_by_coin.is_empty();

    for (i, coin_meta) in universe.iter().enumerate() {
        let Some(spot_index) = coin_meta.get("index").and_then(Value::as_u64) else {
            continue;
        };
        let pair_name = coin_meta.get("name").and_then(Value::as_str);
        if format!("@{spot_index}") != symbol && pair_name != Some(symbol) {
            continue;
        }
        let ctx_val = ctxs_by_coin
            .get(symbol)
            .copied()
            .or_else(|| pair_name.and_then(|name| ctxs_by_coin.get(name).copied()))
            .or_else(|| {
                ctxs.get(i).filter(|ctx| {
                    spot_context_value_matches_symbol(
                        ctx,
                        symbol,
                        pair_name,
                        allow_unkeyed_fallback,
                    )
                })
            })?;
        return serde_json::from_value::<AssetContext>(ctx_val.clone()).ok();
    }

    None
}

fn spot_contexts_by_coin(ctxs: &[Value]) -> HashMap<&str, &Value> {
    ctxs.iter()
        .filter_map(|ctx| {
            let coin = ctx.get("coin").and_then(Value::as_str)?;
            Some((coin, ctx))
        })
        .collect()
}

fn spot_context_value_matches_symbol(
    ctx: &Value,
    symbol_key: &str,
    pair_name: Option<&str>,
    allow_unkeyed_fallback: bool,
) -> bool {
    match ctx.get("coin").and_then(Value::as_str) {
        Some(coin) => coin == symbol_key || pair_name == Some(coin),
        None => allow_unkeyed_fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_chart_asset_context, parse_spot_chart_asset_context,
        validate_spot_chart_asset_context_response,
    };
    use serde_json::json;

    fn hip3_response() -> serde_json::Value {
        json!([
            {
                "universe": [
                    { "name": "xyz:TSLA" },
                    { "name": "xyz:NVDA" }
                ]
            },
            [
                { "funding": "0.0000125", "openInterest": "100.0", "dayNtlVlm": "111.0",
                  "dayBaseVlm": "2.0", "markPx": "400.0", "midPx": "400.5",
                  "oraclePx": "399.5", "prevDayPx": "395.0", "impactPxs": ["400.0", "401.0"] },
                { "funding": "0.0000125", "openInterest": "11560.744", "dayNtlVlm": "987654.0",
                  "dayBaseVlm": "33.0", "markPx": "120.0", "midPx": "120.1",
                  "oraclePx": "119.9", "prevDayPx": "118.0", "impactPxs": ["120.0", "120.2"] }
            ]
        ])
    }

    #[test]
    fn rejects_error_shaped_spot_context_response() {
        for invalid in [
            json!({ "error": "rate limited" }),
            json!([{}, []]),
            json!([{ "universe": [{ "name": "@107", "index": 107 }] }, {}]),
        ] {
            assert!(validate_spot_chart_asset_context_response(&invalid).is_err());
        }
    }

    #[test]
    fn parses_hip3_context_by_full_dex_coin_name() {
        let resp = hip3_response();
        let ctx = parse_chart_asset_context(&resp, "xyz:NVDA", Some("xyz"))
            .expect("context for xyz:NVDA");
        assert_eq!(ctx.open_interest.as_deref(), Some("11560.744"));
        assert_eq!(ctx.day_ntl_vlm.as_deref(), Some("987654.0"));
        assert_eq!(ctx.day_base_vlm.as_deref(), Some("33.0"));
        assert!(ctx.funding.is_some());
    }

    #[test]
    fn matches_bare_universe_names_via_dex_reprefix() {
        // Some per-dex responses name coins bare ("NVDA"); they must still match
        // the canonical "xyz:NVDA" chart symbol after re-prefixing.
        let resp = json!([
            { "universe": [ { "name": "TSLA" }, { "name": "NVDA" } ] },
            [
                { "openInterest": "1.0", "dayNtlVlm": "2.0" },
                { "openInterest": "11560.744", "dayNtlVlm": "987654.0" }
            ]
        ]);
        let ctx = parse_chart_asset_context(&resp, "xyz:NVDA", Some("xyz"))
            .expect("re-prefixed match for xyz:NVDA");
        assert_eq!(ctx.open_interest.as_deref(), Some("11560.744"));
    }

    #[test]
    fn parses_main_dex_context_without_dex_prefix() {
        let resp = json!([
            { "universe": [ { "name": "BTC" }, { "name": "ETH" } ] },
            [
                { "openInterest": "5000.0", "dayNtlVlm": "9.0" },
                { "openInterest": "6000.0", "dayNtlVlm": "8.0" }
            ]
        ]);
        let ctx = parse_chart_asset_context(&resp, "ETH", None).expect("context for ETH");
        assert_eq!(ctx.open_interest.as_deref(), Some("6000.0"));
    }

    #[test]
    fn returns_none_when_symbol_absent_from_universe() {
        let resp = hip3_response();
        assert!(parse_chart_asset_context(&resp, "xyz:AAPL", Some("xyz")).is_none());
    }

    #[test]
    fn returns_none_for_malformed_response() {
        assert!(parse_chart_asset_context(&json!({}), "BTC", None).is_none());
        assert!(parse_chart_asset_context(&json!([{}]), "BTC", None).is_none());
    }

    #[test]
    fn parses_spot_context_by_at_index() {
        let resp = json!([
            {
                "universe": [
                    { "name": "PURR/USDC", "index": 0 },
                    { "name": "HYPE/USDC", "index": 107 }
                ]
            },
            [
                { "midPx": "1.0", "prevDayPx": "0.9", "dayNtlVlm": "1234.0",
                  "dayBaseVlm": "567.0" },
                { "midPx": "62.1", "prevDayPx": "60.0", "dayNtlVlm": "987654.0",
                  "dayBaseVlm": "15555.0" }
            ]
        ]);

        let ctx = parse_spot_chart_asset_context(&resp, "@107").expect("spot context");

        assert_eq!(ctx.mid_px.as_deref(), Some("62.1"));
        assert_eq!(ctx.day_ntl_vlm.as_deref(), Some("987654.0"));
        assert_eq!(ctx.day_base_vlm.as_deref(), Some("15555.0"));
        assert!(ctx.funding.is_none());
        assert!(ctx.open_interest.is_none());
    }

    #[test]
    fn parses_spot_context_by_api_pair_name() {
        // The canonical pair is keyed by its API name ("PURR/USDC"), not by
        // its "@{index}" form, and must still resolve its spot context.
        let resp = json!([
            {
                "universe": [
                    { "name": "PURR/USDC", "index": 0 },
                    { "name": "HYPE/USDC", "index": 107 }
                ]
            },
            [
                { "midPx": "1.0", "prevDayPx": "0.9", "dayNtlVlm": "1234.0",
                  "dayBaseVlm": "567.0" },
                { "midPx": "62.1", "prevDayPx": "60.0", "dayNtlVlm": "987654.0",
                  "dayBaseVlm": "15555.0" }
            ]
        ]);

        let ctx = parse_spot_chart_asset_context(&resp, "PURR/USDC").expect("spot context");

        assert_eq!(ctx.mid_px.as_deref(), Some("1.0"));
        assert_eq!(ctx.prev_day_px.as_deref(), Some("0.9"));
        assert_eq!(ctx.day_ntl_vlm.as_deref(), Some("1234.0"));
    }

    #[test]
    fn parses_spot_context_by_context_coin_when_universe_position_differs_from_index() {
        let resp = json!([
            {
                "universe": [
                    { "name": "@142", "index": 142 }
                ]
            },
            [
                { "coin": "@140", "midPx": "0.000068", "prevDayPx": "0.000037" },
                { "coin": "@142", "midPx": "60105.5", "prevDayPx": "58322.0",
                  "dayNtlVlm": "32176298.0", "dayBaseVlm": "546.48611" }
            ]
        ]);

        let ctx = parse_spot_chart_asset_context(&resp, "@142").expect("spot context");

        assert_eq!(ctx.mid_px.as_deref(), Some("60105.5"));
        assert_eq!(ctx.prev_day_px.as_deref(), Some("58322.0"));
        assert_eq!(ctx.day_ntl_vlm.as_deref(), Some("32176298.0"));
        assert_eq!(ctx.day_base_vlm.as_deref(), Some("546.48611"));
    }

    #[test]
    fn spot_context_returns_none_when_symbol_absent_or_malformed() {
        let resp = json!([
            { "universe": [ { "name": "PURR/USDC", "index": 0 } ] },
            [ { "midPx": "1.0" } ]
        ]);

        assert!(parse_spot_chart_asset_context(&resp, "@107").is_none());
        assert!(parse_spot_chart_asset_context(&json!({}), "@107").is_none());
        assert!(parse_spot_chart_asset_context(&json!([{}]), "@107").is_none());
    }
}

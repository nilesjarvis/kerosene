use super::{API_URL, CLIENT};
use crate::account::AssetContext;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Chart Asset Context REST Fallback
// ---------------------------------------------------------------------------
//
// The chart header's 24h-volume and open-interest metrics are populated from a
// chart's `asset_ctx`, which normally arrives over the `activeAssetCtx`
// WebSocket stream. When that stream does not deliver context for a symbol
// (notably HIP-3 builder-deployed perps, keyed `dex:coin`, when a relaying
// proxy does not forward their `activeAssetCtx`, or during reconnect gaps),
// the header silently blanks because — unlike candles — there was no REST
// fallback. This module provides one, reusing the same per-dex
// `metaAndAssetCtxs` request the watchlist/screener already rely on.

/// Fetch the live [`AssetContext`] for a single chart symbol via the REST
/// `metaAndAssetCtxs` endpoint.
///
/// For a HIP-3 `dex:coin` symbol the `dex` is split out and sent as the
/// `metaAndAssetCtxs` `dex` parameter (the main perp dex omits it). Returns
/// `Ok(None)` for symbols that have no perp asset context here (spot `@`,
/// composite `#`, or a coin absent from the universe).
pub async fn fetch_chart_asset_context(symbol: String) -> Result<Option<AssetContext>, String> {
    if symbol.is_empty() || symbol.starts_with('@') || symbol.starts_with('#') {
        return Ok(None);
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

#[cfg(test)]
mod tests {
    use super::parse_chart_asset_context;
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
}

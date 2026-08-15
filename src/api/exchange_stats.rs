use super::{API_URL, CLIENT};
use crate::helpers::parse_finite_json_number;
use serde_json::Value;
use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// Exchange Stats
// ---------------------------------------------------------------------------

/// Rolling exchange-wide statistics assembled from Hyperliquid asset contexts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ExchangeStats {
    /// Sum of `dayNtlVlm` across main perps, every HIP-3 DEX, and spot.
    pub(crate) volume_24h_notional_usd: f64,
}

/// Fetch a complete rolling 24-hour notional-volume snapshot.
///
/// Hyperliquid exposes volume per market rather than as a single exchange-wide
/// field. `perpDexs` supplies the current HIP-3 DEX list, and each
/// `metaAndAssetCtxs`/`spotMetaAndAssetCtxs` response supplies `dayNtlVlm` for
/// the markets in that family. Every family must succeed so the UI never shows
/// a partial total as though it covered the whole exchange.
pub(crate) async fn fetch_exchange_stats() -> Result<ExchangeStats, String> {
    let client = CLIENT.clone();
    let dex_response = post_info(
        client.clone(),
        serde_json::json!({ "type": "perpDexs" }),
        "perpDexs",
    )
    .await?;
    let dex_names = parse_perp_dex_names(&dex_response)?;

    let mut families = vec![
        (
            "main perps".to_string(),
            serde_json::json!({ "type": "metaAndAssetCtxs" }),
        ),
        (
            "spot".to_string(),
            serde_json::json!({ "type": "spotMetaAndAssetCtxs" }),
        ),
    ];
    families.extend(dex_names.into_iter().map(|dex| {
        (
            format!("HIP-3 dex {dex}"),
            serde_json::json!({ "type": "metaAndAssetCtxs", "dex": dex }),
        )
    }));

    let requests = families.into_iter().map(|(label, body)| {
        let client = client.clone();
        async move {
            let response = post_info(client, body, &label).await?;
            parse_asset_context_volume(&response).map_err(|error| format!("{label}: {error}"))
        }
    });
    let family_volumes = futures::future::join_all(requests).await;

    let mut volume_24h_notional_usd = 0.0;
    for family_volume in family_volumes {
        volume_24h_notional_usd += family_volume?;
        if !volume_24h_notional_usd.is_finite() {
            return Err("exchange 24h volume overflowed".to_string());
        }
    }

    Ok(ExchangeStats {
        volume_24h_notional_usd,
    })
}

async fn post_info(client: reqwest::Client, body: Value, label: &str) -> Result<Value, String> {
    client
        .post(API_URL)
        .json(&body)
        .send()
        .await
        .map_err(|error| format!("{label} request failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("{label} HTTP error: {error}"))?
        .json()
        .await
        .map_err(|error| format!("{label} parse failed: {error}"))
}

fn parse_perp_dex_names(response: &Value) -> Result<Vec<String>, String> {
    let entries = response
        .as_array()
        .ok_or_else(|| "perpDexs schema invalid: expected array".to_string())?;
    let mut names = BTreeSet::new();

    for entry in entries {
        if entry.is_null() {
            continue;
        }
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| "perpDexs schema invalid: missing dex name".to_string())?;
        names.insert(name.to_string());
    }

    Ok(names.into_iter().collect())
}

fn parse_asset_context_volume(response: &Value) -> Result<f64, String> {
    let pair = response
        .as_array()
        .filter(|pair| pair.len() == 2)
        .ok_or_else(|| "expected [meta, contexts] array".to_string())?;
    let contexts = pair[1]
        .as_array()
        .ok_or_else(|| "expected contexts array".to_string())?;
    if contexts.is_empty() {
        return Err("contexts array was empty".to_string());
    }

    let mut volume = 0.0;
    for context in contexts {
        let day_volume = context
            .get("dayNtlVlm")
            .and_then(parse_finite_json_number)
            .filter(|value| *value >= 0.0)
            .ok_or_else(|| "context had invalid dayNtlVlm".to_string())?;
        volume += day_volume;
        if !volume.is_finite() {
            return Err("context volume sum overflowed".to_string());
        }
    }

    Ok(volume)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perp_dex_names_skip_main_and_are_deduplicated() {
        let response = serde_json::json!([
            null,
            { "name": "xyz" },
            { "name": "cash" },
            { "name": "xyz" }
        ]);

        assert_eq!(
            parse_perp_dex_names(&response).expect("valid dex response"),
            vec!["cash".to_string(), "xyz".to_string()]
        );
    }

    #[test]
    fn asset_context_volume_sums_rolling_notional_volume() {
        let response = serde_json::json!([
            { "universe": [{ "name": "BTC" }, { "name": "ETH" }] },
            [
                { "dayNtlVlm": "1250.5" },
                { "dayNtlVlm": 749.5 }
            ]
        ]);

        assert_eq!(
            parse_asset_context_volume(&response).expect("valid context response"),
            2_000.0
        );
    }

    #[test]
    fn asset_context_volume_rejects_partial_or_invalid_totals() {
        let missing_volume = serde_json::json!([
            { "universe": [{ "name": "BTC" }] },
            [{ "markPx": "100" }]
        ]);
        let negative_volume = serde_json::json!([
            { "universe": [{ "name": "BTC" }] },
            [{ "dayNtlVlm": "-1" }]
        ]);

        assert!(parse_asset_context_volume(&missing_volume).is_err());
        assert!(parse_asset_context_volume(&negative_volume).is_err());
    }
}

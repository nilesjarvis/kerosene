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
    /// Sum of `openInterest * markPx` across main perps and every HIP-3 DEX.
    pub(crate) open_interest_notional_usd: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssetContextFamily {
    Perp,
    Spot,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct AssetContextStats {
    volume_24h_notional_usd: f64,
    open_interest_notional_usd: f64,
}

/// Fetch a complete rolling exchange-stat snapshot.
///
/// Hyperliquid exposes volume per market rather than as a single exchange-wide
/// field. `perpDexs` supplies the current HIP-3 DEX list, and each
/// `metaAndAssetCtxs`/`spotMetaAndAssetCtxs` response supplies `dayNtlVlm` for
/// the markets in that family. Perp contexts also supply base-unit
/// `openInterest` and `markPx`, which are multiplied for notional open
/// interest. Every family must succeed so the UI never shows a partial total as
/// though it covered the whole exchange.
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
            AssetContextFamily::Perp,
        ),
        (
            "spot".to_string(),
            serde_json::json!({ "type": "spotMetaAndAssetCtxs" }),
            AssetContextFamily::Spot,
        ),
    ];
    families.extend(dex_names.into_iter().map(|dex| {
        (
            format!("HIP-3 dex {dex}"),
            serde_json::json!({ "type": "metaAndAssetCtxs", "dex": dex }),
            AssetContextFamily::Perp,
        )
    }));

    let requests = families.into_iter().map(|(label, body, family)| {
        let client = client.clone();
        async move {
            let response = post_info(client, body, &label).await?;
            parse_asset_context_stats(&response, family)
                .map_err(|error| format!("{label}: {error}"))
        }
    });
    let family_stats = futures::future::join_all(requests).await;

    let mut exchange_stats = ExchangeStats {
        volume_24h_notional_usd: 0.0,
        open_interest_notional_usd: 0.0,
    };
    for stats in family_stats {
        let stats = stats?;
        exchange_stats.volume_24h_notional_usd += stats.volume_24h_notional_usd;
        exchange_stats.open_interest_notional_usd += stats.open_interest_notional_usd;
        if !exchange_stats.volume_24h_notional_usd.is_finite() {
            return Err("exchange 24h volume overflowed".to_string());
        }
        if !exchange_stats.open_interest_notional_usd.is_finite() {
            return Err("exchange open interest overflowed".to_string());
        }
    }

    Ok(exchange_stats)
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

fn parse_asset_context_stats(
    response: &Value,
    family: AssetContextFamily,
) -> Result<AssetContextStats, String> {
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

    let mut stats = AssetContextStats::default();
    for context in contexts {
        let day_volume = context
            .get("dayNtlVlm")
            .and_then(parse_finite_json_number)
            .filter(|value| *value >= 0.0)
            .ok_or_else(|| "context had invalid dayNtlVlm".to_string())?;
        stats.volume_24h_notional_usd += day_volume;
        if !stats.volume_24h_notional_usd.is_finite() {
            return Err("context volume sum overflowed".to_string());
        }

        if family == AssetContextFamily::Perp {
            let open_interest = context
                .get("openInterest")
                .and_then(parse_finite_json_number)
                .filter(|value| *value >= 0.0)
                .ok_or_else(|| "context had invalid openInterest".to_string())?;
            let mark_price = context
                .get("markPx")
                .and_then(parse_finite_json_number)
                .filter(|value| *value > 0.0)
                .ok_or_else(|| "context had invalid markPx".to_string())?;
            stats.open_interest_notional_usd += open_interest * mark_price;
            if !stats.open_interest_notional_usd.is_finite() {
                return Err("context open interest sum overflowed".to_string());
            }
        }
    }

    Ok(stats)
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
    fn perp_asset_context_stats_sum_volume_and_notional_open_interest() {
        let response = serde_json::json!([
            { "universe": [{ "name": "BTC" }, { "name": "ETH" }] },
            [
                { "dayNtlVlm": "1250.5", "openInterest": "10", "markPx": "100" },
                { "dayNtlVlm": 749.5, "openInterest": 5, "markPx": 200 }
            ]
        ]);

        assert_eq!(
            parse_asset_context_stats(&response, AssetContextFamily::Perp)
                .expect("valid context response"),
            AssetContextStats {
                volume_24h_notional_usd: 2_000.0,
                open_interest_notional_usd: 2_000.0,
            }
        );
    }

    #[test]
    fn spot_asset_context_stats_do_not_require_open_interest() {
        let response = serde_json::json!([
            { "universe": [{ "name": "HYPE/USDC" }] },
            [{ "dayNtlVlm": "1250.5" }]
        ]);

        assert_eq!(
            parse_asset_context_stats(&response, AssetContextFamily::Spot)
                .expect("valid spot context response"),
            AssetContextStats {
                volume_24h_notional_usd: 1_250.5,
                open_interest_notional_usd: 0.0,
            }
        );
    }

    #[test]
    fn asset_context_stats_reject_partial_or_invalid_totals() {
        let missing_volume = serde_json::json!([
            { "universe": [{ "name": "BTC" }] },
            [{ "openInterest": "1", "markPx": "100" }]
        ]);
        let negative_volume = serde_json::json!([
            { "universe": [{ "name": "BTC" }] },
            [{ "dayNtlVlm": "-1", "openInterest": "1", "markPx": "100" }]
        ]);
        let missing_open_interest = serde_json::json!([
            { "universe": [{ "name": "BTC" }] },
            [{ "dayNtlVlm": "1", "markPx": "100" }]
        ]);
        let invalid_mark_price = serde_json::json!([
            { "universe": [{ "name": "BTC" }] },
            [{ "dayNtlVlm": "1", "openInterest": "1", "markPx": "0" }]
        ]);

        for response in [
            missing_volume,
            negative_volume,
            missing_open_interest,
            invalid_mark_price,
        ] {
            assert!(parse_asset_context_stats(&response, AssetContextFamily::Perp).is_err());
        }
    }
}

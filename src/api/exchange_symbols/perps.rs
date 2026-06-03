use super::{ExchangeSymbol, MarketType};
use serde_json::Value;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Perp Symbols
// ---------------------------------------------------------------------------

pub(super) fn append_perp_symbols(
    symbols: &mut Vec<ExchangeSymbol>,
    metas_raw: &Value,
    annotations_raw: &Value,
    dexs_raw: &Value,
) -> Result<(), String> {
    let dex_offsets = dex_offsets_from(dexs_raw);
    let annotation_map = annotation_map_from(annotations_raw);
    let empty_vec = Vec::new();
    let dexes = metas_raw.as_array().ok_or("Expected array of dex metas")?;

    for (dex_idx, dex_meta) in dexes.iter().enumerate() {
        let offset = dex_offsets.get(dex_idx).copied().unwrap_or(0);
        let universe = dex_meta
            .get("universe")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);
        let collateral_token = dex_meta
            .get("collateralToken")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        for (asset_idx, asset) in universe.iter().enumerate() {
            if asset
                .get("isDelisted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                continue;
            }

            let name = asset
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            if name.is_empty() {
                continue;
            }

            let sz_decimals = asset
                .get("szDecimals")
                .and_then(|v| v.as_u64())
                .unwrap_or(2) as u32;

            let max_leverage = asset
                .get("maxLeverage")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as u32;

            let only_isolated = asset
                .get("onlyIsolated")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                || margin_mode_disallows_cross(asset);

            let ticker = name.split(':').nth(1).unwrap_or(&name).to_string();
            let annotation = annotation_map.get(&name);

            let category = annotation
                .and_then(|a| a.get("category"))
                .and_then(|v| v.as_str())
                .unwrap_or("crypto")
                .to_string();

            let display_name = annotation
                .and_then(|a| a.get("displayName"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let keywords = annotation
                .and_then(|a| a.get("keywords"))
                .and_then(|v| v.as_array())
                .map(|kw_arr| {
                    kw_arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            symbols.push(ExchangeSymbol {
                key: name,
                ticker,
                category,
                display_name,
                keywords,
                asset_index: offset + asset_idx as u32,
                collateral_token: Some(collateral_token),
                sz_decimals,
                max_leverage,
                only_isolated,
                market_type: MarketType::Perp,
                outcome: None,
            });
        }
    }

    Ok(())
}

fn dex_offsets_from(dexs_raw: &Value) -> Vec<u32> {
    let mut dex_offsets = Vec::new();
    if let Some(dexs) = dexs_raw.as_array() {
        for (i, _dex) in dexs.iter().enumerate() {
            if i == 0 {
                dex_offsets.push(0);
            } else {
                dex_offsets.push(110_000 + (i as u32 - 1) * 10_000);
            }
        }
    }
    dex_offsets
}

fn margin_mode_disallows_cross(asset: &Value) -> bool {
    asset
        .get("marginMode")
        .and_then(|v| v.as_str())
        .map(|mode| {
            matches!(
                mode.to_ascii_lowercase().as_str(),
                "strictisolated" | "nocross" | "onlyisolated"
            )
        })
        .unwrap_or(false)
}

fn annotation_map_from(annotations_raw: &Value) -> HashMap<String, Value> {
    let mut annotation_map = HashMap::new();
    if let Some(pairs) = annotations_raw.as_array() {
        for pair in pairs {
            if let Some(arr) = pair.as_array()
                && arr.len() >= 2
                && let Some(coin_key) = arr[0].as_str()
            {
                annotation_map.insert(coin_key.to_string(), arr[1].clone());
            }
        }
    }
    annotation_map
}

#[cfg(test)]
mod tests {
    use super::margin_mode_disallows_cross;

    #[test]
    fn margin_mode_strict_isolated_disallows_cross() {
        assert!(margin_mode_disallows_cross(&serde_json::json!({
            "marginMode": "strictIsolated"
        })));
    }

    #[test]
    fn margin_mode_no_cross_disallows_cross() {
        assert!(margin_mode_disallows_cross(&serde_json::json!({
            "marginMode": "noCross"
        })));
    }

    #[test]
    fn unknown_margin_mode_keeps_cross_allowed() {
        assert!(!margin_mode_disallows_cross(&serde_json::json!({
            "marginMode": "cross"
        })));
        assert!(!margin_mode_disallows_cross(&serde_json::json!({})));
    }
}

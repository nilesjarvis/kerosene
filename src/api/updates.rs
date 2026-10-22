use super::shared_reads::public_info;
use crate::config::updates::{
    AssetParameters, MAX_UPDATE_ASSETS, MAX_UPDATE_MARGIN_TIERS, MarginTier, MarketMarginMode,
    ParameterDecimal,
};
use crate::market_state::updates::UpdatesSnapshot;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) async fn fetch_updates_snapshot() -> UpdatesSnapshot {
    let (parameters, caps) = futures::join!(
        public_info(serde_json::json!({"type": "allPerpMetas"})),
        public_info(serde_json::json!({"type": "perpDexs"})),
    );
    UpdatesSnapshot {
        parameters: parameters.and_then(parse_parameters),
        caps: caps.and_then(parse_caps),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PerpMeta {
    universe: Vec<Perp>,
    margin_tables: Vec<(u32, MarginTable)>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Perp {
    name: String,
    max_leverage: u32,
    margin_table_id: Option<u32>,
    margin_mode: Option<MarketMarginMode>,
    #[serde(default)]
    only_isolated: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarginTable {
    margin_tiers: Vec<MarginTier>,
}

fn valid_key(key: &str) -> bool {
    !key.is_empty() && key.len() <= 128 && !key.chars().any(|c| c.is_whitespace() || c.is_control())
}

fn valid_tiers(tiers: &[MarginTier]) -> bool {
    !tiers.is_empty()
        && tiers.len() <= 100
        && tiers[0].lower_bound.as_str() == "0"
        && tiers.iter().all(|tier| tier.max_leverage > 0)
        && tiers.windows(2).all(|pair| {
            pair[0].lower_bound < pair[1].lower_bound
                && pair[0].max_leverage >= pair[1].max_leverage
        })
}

fn parse_parameters(raw: Value) -> Result<BTreeMap<String, AssetParameters>, String> {
    let metas: Vec<PerpMeta> =
        serde_json::from_value(raw).map_err(|_| "Invalid market parameter metadata".to_string())?;
    if metas.first().is_none_or(|meta| meta.universe.is_empty()) {
        return Err("Empty market parameter metadata".into());
    }
    let mut result = BTreeMap::new();
    let mut tier_count = 0usize;
    for (dex_index, meta) in metas.into_iter().enumerate() {
        let mut tables = BTreeMap::new();
        for (id, table) in meta.margin_tables {
            if !valid_tiers(&table.margin_tiers) || tables.insert(id, table.margin_tiers).is_some()
            {
                return Err("Invalid market margin tables".into());
            }
        }
        for asset in meta.universe {
            if !valid_key(&asset.name)
                || asset.max_leverage == 0
                || (dex_index == 0) == asset.name.contains(':')
                || asset.name.split(':').any(str::is_empty)
                || asset.name.matches(':').count() > 1
            {
                return Err("Invalid market parameter identity".into());
            }
            let table_id = asset.margin_table_id.unwrap_or(asset.max_leverage);
            let tiers = match tables.get(&table_id) {
                Some(tiers) => tiers,
                // IDs <= 50 encode a flat leverage table, often omitted from marginTables.
                None if table_id == asset.max_leverage && table_id <= 50 => {
                    tier_count = tier_count.saturating_add(1);
                    if tier_count > MAX_UPDATE_MARGIN_TIERS {
                        return Err("Market parameter tier budget exceeded".into());
                    }
                    let parameters = AssetParameters {
                        max_leverage: asset.max_leverage,
                        margin_tiers: vec![MarginTier {
                            lower_bound: ParameterDecimal::try_from("0".to_string())?,
                            max_leverage: asset.max_leverage,
                        }],
                        margin_mode: asset
                            .margin_mode
                            .or_else(|| (!asset.only_isolated).then_some(MarketMarginMode::Normal)),
                    };
                    if result.insert(asset.name, parameters).is_some()
                        || result.len() > MAX_UPDATE_ASSETS
                    {
                        return Err("Duplicate or excessive market parameters".into());
                    }
                    continue;
                }
                None => return Err("Missing market margin table".into()),
            };
            tier_count = tier_count.saturating_add(tiers.len());
            if tier_count > MAX_UPDATE_MARGIN_TIERS {
                return Err("Market parameter tier budget exceeded".into());
            }
            let parameters = AssetParameters {
                max_leverage: asset.max_leverage,
                margin_tiers: tiers.clone(),
                margin_mode: asset
                    .margin_mode
                    .or_else(|| (!asset.only_isolated).then_some(MarketMarginMode::Normal)),
            };
            if result.insert(asset.name, parameters).is_some() || result.len() > MAX_UPDATE_ASSETS {
                return Err("Duplicate or excessive market parameters".into());
            }
        }
    }
    Ok(result)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PerpDex {
    name: String,
    asset_to_streaming_oi_cap: Vec<(String, ParameterDecimal)>,
}

fn parse_caps(raw: Value) -> Result<BTreeMap<String, ParameterDecimal>, String> {
    let dexes: Vec<Option<PerpDex>> =
        serde_json::from_value(raw).map_err(|_| "Invalid market cap metadata".to_string())?;
    if !matches!(dexes.first(), Some(None)) {
        return Err("Missing native DEX marker".into());
    }
    let mut names = BTreeSet::new();
    let mut result = BTreeMap::new();
    for dex in dexes.into_iter().skip(1) {
        let dex = dex.ok_or("Missing builder DEX metadata")?;
        if !valid_key(&dex.name) || dex.name.contains(':') || !names.insert(dex.name.clone()) {
            return Err("Invalid builder DEX identity".into());
        }
        for (coin, cap) in dex.asset_to_streaming_oi_cap {
            if !valid_key(&coin) {
                return Err("Invalid cap asset identity".into());
            }
            let key = match coin.split_once(':') {
                Some((prefix, name))
                    if prefix == dex.name && !name.is_empty() && !name.contains(':') =>
                {
                    coin
                }
                Some(_) => return Err("Mismatched cap DEX identity".into()),
                None => format!("{}:{coin}", dex.name),
            };
            if result.insert(key, cap).is_some() || result.len() > MAX_UPDATE_ASSETS {
                return Err("Duplicate or excessive market caps".into());
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests;

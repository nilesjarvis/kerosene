use crate::account::AssetContext;
use serde_json::Value;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

pub(super) async fn fetch_spot_chart_asset_context(
    symbol: String,
) -> Result<Option<AssetContext>, String> {
    let contexts = fetch_spot_chart_asset_contexts(vec![symbol.clone()]).await?;
    Ok(contexts
        .into_iter()
        .find_map(|(context_symbol, context)| (context_symbol == symbol).then_some(context)))
}

/// Fetch all requested spot chart contexts with one request and one response index.
pub(crate) async fn fetch_spot_chart_asset_contexts(
    symbols: Vec<String>,
) -> Result<Vec<(String, AssetContext)>, String> {
    if symbols.is_empty() {
        return Ok(Vec::new());
    }
    let response = crate::api::shared_reads::public_info(
        serde_json::json!({ "type": "spotMetaAndAssetCtxs" }),
    )
    .await?;
    parse_spot_chart_asset_contexts(&response, symbols)
}

fn parse_spot_chart_asset_contexts(
    response: &Value,
    symbols: Vec<String>,
) -> Result<Vec<(String, AssetContext)>, String> {
    let lookup = SpotContextLookup::new(response)?;
    let mut seen = HashSet::new();
    Ok(symbols
        .into_iter()
        .filter(|symbol| seen.insert(symbol.clone()))
        .filter_map(|symbol| lookup.get(&symbol).map(|context| (symbol, context)))
        .collect())
}

struct SpotContextLookup<'a> {
    universe_by_symbol: HashMap<Cow<'a, str>, (usize, Option<&'a str>)>,
    contexts: &'a [Value],
    contexts_by_coin: HashMap<&'a str, &'a Value>,
}

impl<'a> SpotContextLookup<'a> {
    fn new(response: &'a Value) -> Result<Self, String> {
        let response_parts = response
            .as_array()
            .filter(|parts| parts.len() == 2)
            .ok_or_else(|| {
                "spotMetaAndAssetCtxs schema invalid: expected [meta, contexts]".to_string()
            })?;
        let universe = response_parts[0]
            .get("universe")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                "spotMetaAndAssetCtxs schema invalid: missing meta universe".to_string()
            })?;
        let contexts = response_parts[1].as_array().ok_or_else(|| {
            "spotMetaAndAssetCtxs schema invalid: contexts must be an array".to_string()
        })?;
        if universe.is_empty() || contexts.is_empty() {
            return Err(
                "spotMetaAndAssetCtxs schema invalid: empty universe or contexts".to_string(),
            );
        }

        // Universe aliases choose the first matching row; duplicate context
        // coins choose the last payload.
        let mut universe_by_symbol = HashMap::new();
        for (position, coin_meta) in universe.iter().enumerate() {
            let Some(index) = coin_meta.get("index").and_then(Value::as_u64) else {
                continue;
            };
            let pair_name = coin_meta.get("name").and_then(Value::as_str);
            let entry = (position, pair_name);
            universe_by_symbol
                .entry(Cow::Owned(format!("@{index}")))
                .or_insert(entry);
            if let Some(name) = pair_name {
                universe_by_symbol
                    .entry(Cow::Borrowed(name))
                    .or_insert(entry);
            }
        }
        let contexts_by_coin = contexts
            .iter()
            .filter_map(|context| {
                context
                    .get("coin")
                    .and_then(Value::as_str)
                    .map(|coin| (coin, context))
            })
            .collect();
        Ok(Self {
            universe_by_symbol,
            contexts,
            contexts_by_coin,
        })
    }

    fn get(&self, symbol: &str) -> Option<AssetContext> {
        let &(position, pair_name) = self.universe_by_symbol.get(symbol)?;
        let context = self
            .contexts_by_coin
            .get(symbol)
            .copied()
            .or_else(|| pair_name.and_then(|name| self.contexts_by_coin.get(name).copied()))
            .or_else(|| {
                self.contexts.get(position).filter(|context| {
                    spot_context_value_matches_symbol(
                        context,
                        symbol,
                        pair_name,
                        self.contexts_by_coin.is_empty(),
                    )
                })
            })?;
        serde_json::from_value::<AssetContext>(context.clone()).ok()
    }
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
mod tests;

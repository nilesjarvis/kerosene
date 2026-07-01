use super::model::WatchlistContext;
use crate::helpers::parse_finite_json_number;
use serde_json::{Map, Value};
use std::collections::HashMap;

pub(super) fn insert_empty_context(map: &mut HashMap<String, WatchlistContext>, symbol: &str) {
    map.insert(
        symbol.to_string(),
        WatchlistContext {
            funding: None,
            prev_day_px: None,
            day_vlm: None,
        },
    );
}

pub(super) fn append_perp_contexts(
    resp: Value,
    dex: Option<&str>,
    map: &mut HashMap<String, WatchlistContext>,
) -> Result<usize, String> {
    let arr = resp
        .as_array()
        .ok_or_else(|| "expected [meta, contexts] array".to_string())?;
    if arr.len() != 2 {
        return Err("expected [meta, contexts] array with two entries".to_string());
    }
    let meta = arr[0]
        .as_object()
        .ok_or_else(|| "expected meta object".to_string())?;
    let ctxs = arr[1]
        .as_array()
        .ok_or_else(|| "expected contexts array".to_string())?;
    let universe = meta
        .get("universe")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "expected meta.universe array".to_string())?;
    if ctxs.len() < universe.len() {
        return Err("contexts array shorter than universe".to_string());
    }

    let mut parsed = Vec::new();
    for (i, coin_meta) in universe.iter().enumerate() {
        if let Some(name) = coin_meta.get("name").and_then(|n| n.as_str()) {
            let ctx = ctxs
                .get(i)
                .and_then(|v| v.as_object())
                .ok_or_else(|| format!("expected context object for {name}"))?;
            let context = WatchlistContext {
                funding: parse_optional_f64(ctx, "funding"),
                prev_day_px: parse_optional_f64(ctx, "prevDayPx"),
                day_vlm: parse_optional_f64(ctx, "dayNtlVlm"),
            };
            let canonical_key = if name.contains(':') {
                name.to_string()
            } else if let Some(dex) = dex {
                format!("{dex}:{name}")
            } else {
                name.to_string()
            };
            parsed.push((canonical_key, name.to_string(), context));
        }
    }

    let appended = parsed.len();
    for (canonical_key, name, context) in parsed {
        map.insert(canonical_key, context.clone());
        map.entry(name).or_insert(context);
    }

    Ok(appended)
}

pub(super) fn append_spot_contexts(
    resp: Value,
    map: &mut HashMap<String, WatchlistContext>,
) -> Result<usize, String> {
    let arr = resp
        .as_array()
        .ok_or_else(|| "expected [meta, contexts] array".to_string())?;
    if arr.len() != 2 {
        return Err("expected [meta, contexts] array with two entries".to_string());
    }
    let meta = arr[0]
        .as_object()
        .ok_or_else(|| "expected meta object".to_string())?;
    let ctxs = arr[1]
        .as_array()
        .ok_or_else(|| "expected contexts array".to_string())?;
    let universe = meta
        .get("universe")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "expected meta.universe array".to_string())?;
    if ctxs.len() < universe.len() {
        return Err("contexts array shorter than universe".to_string());
    }

    let mut ctxs_by_coin = HashMap::new();
    for ctx_value in ctxs {
        let Some(ctx) = ctx_value.as_object() else {
            continue;
        };
        if let Some(coin) = ctx.get("coin").and_then(|v| v.as_str()) {
            ctxs_by_coin.entry(coin).or_insert(ctx);
        }
    }

    let mut parsed = Vec::new();
    for (i, coin_meta) in universe.iter().enumerate() {
        if let Some(spot_index) = coin_meta.get("index").and_then(|v| v.as_u64()) {
            let symbol_key = format!("@{spot_index}");
            let pair_name = coin_meta.get("name").and_then(|v| v.as_str());
            let allow_unkeyed_fallback = ctxs_by_coin.is_empty();
            let ctx = ctxs.get(i).and_then(|v| v.as_object()).filter(|ctx| {
                spot_context_matches_symbol(ctx, &symbol_key, pair_name, allow_unkeyed_fallback)
            });
            let ctx = ctxs_by_coin
                .get(symbol_key.as_str())
                .copied()
                .or_else(|| pair_name.and_then(|name| ctxs_by_coin.get(name).copied()))
                .or(ctx)
                .ok_or_else(|| format!("expected context object for @{spot_index}"))?;
            let prev_day_px = parse_optional_f64(ctx, "prevDayPx");
            let day_vlm = parse_optional_f64(ctx, "dayNtlVlm");
            parsed.push((
                symbol_key,
                WatchlistContext {
                    funding: None,
                    prev_day_px,
                    day_vlm,
                },
            ));
        }
    }

    let appended = parsed.len();
    map.extend(parsed);

    Ok(appended)
}

fn spot_context_matches_symbol(
    ctx: &Map<String, Value>,
    symbol_key: &str,
    pair_name: Option<&str>,
    allow_unkeyed_fallback: bool,
) -> bool {
    match ctx.get("coin").and_then(|v| v.as_str()) {
        Some(coin) => coin == symbol_key || pair_name == Some(coin),
        None => allow_unkeyed_fallback,
    }
}

fn parse_optional_f64(ctx: &Map<String, Value>, key: &str) -> Option<f64> {
    ctx.get(key).and_then(parse_finite_json_number)
}

#[cfg(test)]
mod tests;

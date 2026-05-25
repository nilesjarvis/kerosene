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
) {
    if let Some(arr) = resp.as_array()
        && arr.len() == 2
        && let (Some(meta), Some(ctxs)) = (arr[0].as_object(), arr[1].as_array())
        && let Some(universe) = meta.get("universe").and_then(|v| v.as_array())
    {
        for (i, coin_meta) in universe.iter().enumerate() {
            if let Some(name) = coin_meta.get("name").and_then(|n| n.as_str())
                && let Some(ctx) = ctxs.get(i).and_then(|v| v.as_object())
            {
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
                map.insert(canonical_key, context.clone());
                map.entry(name.to_string()).or_insert(context);
            }
        }
    }
}

pub(super) fn append_spot_contexts(resp: Value, map: &mut HashMap<String, WatchlistContext>) {
    if let Some(arr) = resp.as_array()
        && arr.len() == 2
        && let (Some(meta), Some(ctxs)) = (arr[0].as_object(), arr[1].as_array())
        && let Some(universe) = meta.get("universe").and_then(|v| v.as_array())
    {
        for (i, coin_meta) in universe.iter().enumerate() {
            if let Some(spot_index) = coin_meta.get("index").and_then(|v| v.as_u64())
                && let Some(ctx) = ctxs.get(i).and_then(|v| v.as_object())
            {
                let prev_day_px = parse_optional_f64(ctx, "prevDayPx");
                let day_vlm = parse_optional_f64(ctx, "dayNtlVlm");
                map.insert(
                    format!("@{}", spot_index),
                    WatchlistContext {
                        funding: None,
                        prev_day_px,
                        day_vlm,
                    },
                );
            }
        }
    }
}

fn parse_optional_f64(ctx: &Map<String, Value>, key: &str) -> Option<f64> {
    ctx.get(key).and_then(parse_finite_json_number)
}

#[cfg(test)]
mod tests;

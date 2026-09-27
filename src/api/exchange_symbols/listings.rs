use super::{CLIENT, post_info_value, spot::append_spot_symbols};
use crate::market_state::listings::{ListingMarket, ListingsSnapshot};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;

/// Two public metadata requests; no account data or third-party credentials.
pub(crate) async fn fetch_listings_snapshot() -> ListingsSnapshot {
    let (perps, spot) = futures::join!(
        post_info_value(CLIENT.clone(), "allPerpMetas"),
        post_info_value(CLIENT.clone(), "spotMeta"),
    );
    ListingsSnapshot {
        perps: perps.and_then(parse_perps),
        spot: spot.and_then(parse_spot),
    }
}

#[derive(Deserialize)]
struct PerpMeta {
    universe: Vec<Perp>,
}

#[derive(Deserialize)]
struct Perp {
    name: String,
    #[serde(default, rename = "isDelisted")]
    inactive: bool,
}

fn parse_perps(raw: Value) -> Result<Vec<ListingMarket>, String> {
    let metas: Vec<PerpMeta> = serde_json::from_value(raw)
        .map_err(|_| "Invalid perpetual listing metadata".to_string())?;
    if metas.first().is_none_or(|meta| meta.universe.is_empty()) {
        return Err("Empty perpetual listing metadata".into());
    }
    let mut seen = HashSet::new();
    let mut markets = Vec::new();
    for (dex, meta) in metas.into_iter().enumerate() {
        for perp in meta.universe {
            if perp.name.trim().is_empty()
                || !seen.insert(perp.name.clone())
                || (dex > 0 && !perp.name.contains(':'))
            {
                return Err("Invalid perpetual listing identity".into());
            }
            markets.push(ListingMarket {
                id: format!("perp:{}", perp.name),
                label: perp
                    .name
                    .split_once(':')
                    .map_or(perp.name.as_str(), |(_, ticker)| ticker)
                    .to_string(),
                key: perp.name,
                active: !perp.inactive,
            });
        }
    }
    Ok(markets)
}

fn parse_spot(raw: Value) -> Result<Vec<ListingMarket>, String> {
    let mut symbols = Vec::new();
    append_spot_symbols(&mut symbols, &raw)?;
    Ok(symbols
        .into_iter()
        .map(|symbol| ListingMarket {
            id: format!("spot:{}", symbol.asset_index),
            label: symbol.display_name.unwrap_or(symbol.ticker),
            key: symbol.key,
            active: true,
        })
        .collect())
}

#[cfg(test)]
mod tests;

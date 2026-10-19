use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;

/// Papertrade instrument identity; never an index into Hyperliquid's asset list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PapertradeMarketId(pub(crate) u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PapertradeMarket {
    pub(crate) id: PapertradeMarketId,
    pub(crate) symbol: String,
    pub(crate) max_leverage: u32,
    pub(crate) status: &'static str,
}

impl PapertradeMarket {
    pub(crate) fn matches_query(&self, normalized_query: &str) -> bool {
        self.symbol.to_ascii_lowercase().contains(normalized_query)
            || self.id.0.to_string() == normalized_query
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PapertradeMarkets {
    pub(crate) indexed_through_block: u64,
    pub(crate) markets: Vec<PapertradeMarket>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TradingState {
    schema_version: u32,
    indexed_through_block: u64,
    trading_paused: bool,
    instruments: Vec<Vec<Value>>,
}

pub(super) fn parse_markets(bytes: &[u8]) -> Result<PapertradeMarkets, String> {
    let state: TradingState = serde_json::from_slice(bytes)
        .map_err(|_| "Papertrade returned an invalid market catalog.".to_string())?;
    if state.schema_version != 1 {
        return Err("Papertrade market schema is not supported.".to_string());
    }
    let mut ids = HashSet::new();
    let mut markets = Vec::with_capacity(state.instruments.len());
    for row in state.instruments {
        let market = parse_instrument(&row, state.trading_paused)
            .ok_or_else(|| "Papertrade returned invalid market metadata.".to_string())?;
        if !ids.insert(market.id) {
            return Err("Papertrade returned duplicate market IDs.".to_string());
        }
        markets.push(market);
    }
    markets.sort_by(|left, right| {
        left.symbol
            .cmp(&right.symbol)
            .then_with(|| left.id.0.cmp(&right.id.0))
    });
    Ok(PapertradeMarkets {
        indexed_through_block: state.indexed_through_block,
        markets,
    })
}

// Schema v1 uses positional instrument tuples. Only catalog fields are consumed;
// these flags are display metadata, not sufficient admission checks for trading.
fn parse_instrument(row: &[Value], trading_paused: bool) -> Option<PapertradeMarket> {
    if row.len() < 23 {
        return None;
    }
    let id = u32::try_from(row[0].as_u64()?).ok()?;
    let symbol = row[1].as_str()?;
    if symbol.is_empty()
        || symbol.len() > 64
        || !symbol
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_/.:".contains(&byte))
    {
        return None;
    }
    let active = row[4].as_bool()?;
    let paused = row[5].as_bool()?;
    let terminal = row[6].as_bool()?;
    let retired = row[7].as_bool()?;
    let close_only = row[8].as_bool()?;
    let max_leverage = u32::try_from(row[14].as_u64()?).ok()?;
    let status = if terminal || retired {
        "Retired"
    } else if !active {
        "Inactive"
    } else if close_only {
        "Close only"
    } else if paused || trading_paused {
        "Paused"
    } else {
        "Perp"
    };
    Some(PapertradeMarket {
        id: PapertradeMarketId(id),
        symbol: symbol.to_string(),
        max_leverage,
        status,
    })
}

#[cfg(test)]
mod tests;

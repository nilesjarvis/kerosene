use super::{ExchangeSymbol, MarketType};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Spot Symbols
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct SpotMetaResponse {
    tokens: Vec<SpotToken>,
    universe: Vec<SpotPair>,
}

#[derive(Deserialize)]
struct SpotToken {
    name: String,
    #[serde(rename = "szDecimals")]
    sz_decimals: u32,
    index: u32,
    #[serde(default, rename = "fullName")]
    full_name: Option<String>,
}

#[derive(Deserialize)]
struct SpotPair {
    name: String,
    tokens: [u32; 2],
    index: u32,
}

/// Parse and append a complete `spotMeta` response.
///
/// Spot metadata determines order asset IDs, size precision, and which token
/// pays for a buy. Treating an error-shaped or partially malformed JSON body as
/// success can therefore create unsafe order parameters. Validation is strict:
/// both top-level collections must be non-empty, indices must be unique, and
/// every pair must reference two tokens present in the same response.
pub(super) fn append_spot_symbols(
    symbols: &mut Vec<ExchangeSymbol>,
    spot_meta: &Value,
) -> Result<(), String> {
    let meta: SpotMetaResponse = serde_json::from_value(spot_meta.clone())
        .map_err(|e| format!("spotMeta schema invalid: {e}"))?;
    if meta.tokens.is_empty() {
        return Err("spotMeta tokens list is empty".to_string());
    }
    if meta.universe.is_empty() {
        return Err("spotMeta universe is empty".to_string());
    }

    let mut token_info: HashMap<u32, (String, u32, Option<String>)> = HashMap::new();
    let mut token_names = HashSet::new();
    for token in meta.tokens {
        let name = token.name.trim();
        if name.is_empty() {
            return Err(format!("spotMeta token {} has an empty name", token.index));
        }
        if name.contains('/') {
            return Err(format!(
                "spotMeta token {} has an invalid pair separator in its name",
                token.index
            ));
        }
        if token.sz_decimals > 8 {
            return Err(format!(
                "spotMeta token {} has unsafe szDecimals {} (expected 0..=8)",
                token.index, token.sz_decimals
            ));
        }
        let normalized_name = name.to_ascii_uppercase();
        if !token_names.insert(normalized_name.clone()) {
            return Err(format!(
                "spotMeta contains duplicate token name {normalized_name}"
            ));
        }
        if token_info
            .insert(
                token.index,
                (name.to_string(), token.sz_decimals, token.full_name),
            )
            .is_some()
        {
            return Err(format!(
                "spotMeta contains duplicate token index {}",
                token.index
            ));
        }
    }

    let mut parsed = Vec::with_capacity(meta.universe.len());
    let mut spot_indices = HashSet::new();
    let mut symbol_keys = HashSet::new();
    for pair in meta.universe {
        if !spot_indices.insert(pair.index) {
            return Err(format!(
                "spotMeta contains duplicate universe index {}",
                pair.index
            ));
        }
        let pair_name = pair.name.trim();
        if pair_name.is_empty() {
            return Err(format!(
                "spotMeta universe index {} has an empty name",
                pair.index
            ));
        }

        let [base_idx, quote_idx] = pair.tokens;
        if base_idx == quote_idx {
            return Err(format!(
                "spotMeta universe index {} uses the same base and quote token {}",
                pair.index, base_idx
            ));
        }
        let (base_name, sz_decimals, full_name) =
            token_info.get(&base_idx).cloned().ok_or_else(|| {
                format!(
                    "spotMeta universe index {} references unknown base token {}",
                    pair.index, base_idx
                )
            })?;
        let quote_name = token_info
            .get(&quote_idx)
            .map(|(name, _, _)| name.clone())
            .ok_or_else(|| {
                format!(
                    "spotMeta universe index {} references unknown quote token {}",
                    pair.index, quote_idx
                )
            })?;

        // The exchange universe is quote-oriented. A USDC-base row is not a
        // user-facing spot market in Kerosene, but it must still be valid.
        if base_name == "USDC" {
            continue;
        }

        let asset_index = 10_000u32.checked_add(pair.index).ok_or_else(|| {
            format!(
                "spotMeta universe index {} cannot be encoded as an order asset",
                pair.index
            )
        })?;
        let indexed_key = format!("@{}", pair.index);
        // Hyperliquid names canonical launch pairs directly (PURR/USDC is
        // the only one today) and uses that name — not "@{index}" — as the
        // coin in allMids, candles, l2Book, open orders, and fills, so it
        // must be the symbol key. Every other pair stays keyed "@{index}".
        let is_api_named = pair_name != indexed_key;
        let token_derived_display = format!("{base_name}/{quote_name}");
        if is_api_named && pair_name != token_derived_display {
            return Err(format!(
                "spotMeta universe index {} name does not match its referenced base and quote tokens",
                pair.index
            ));
        }
        let key = if is_api_named {
            pair_name.to_string()
        } else {
            indexed_key
        };
        if !symbol_keys.insert(key.clone()) {
            return Err(format!("spotMeta contains duplicate symbol key {key}"));
        }
        let display = Some(token_derived_display);

        let mut kw = Vec::new();
        if let Some(fn_name) = &full_name {
            kw.push(fn_name.to_lowercase());
        }
        kw.push("spot".to_string());

        parsed.push(ExchangeSymbol {
            key,
            ticker: base_name,
            category: "spot".to_string(),
            display_name: display,
            keywords: kw,
            asset_index,
            // Reuse this field for the spot quote token. The sizing path must
            // debit the pair's actual quote balance rather than assume USDC.
            collateral_token: Some(quote_idx),
            sz_decimals,
            max_leverage: 1,
            only_isolated: false,
            market_type: MarketType::Spot,
            outcome: None,
        });
    }

    if parsed.is_empty() {
        return Err("spotMeta contains no supported spot markets".to_string());
    }
    symbols.extend(parsed);
    Ok(())
}

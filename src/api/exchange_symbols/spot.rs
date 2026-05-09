use super::{ExchangeSymbol, MarketType};
use serde_json::Value;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Spot Symbols
// ---------------------------------------------------------------------------

pub(super) fn append_spot_symbols(symbols: &mut Vec<ExchangeSymbol>, spot_meta: &Value) {
    let mut token_info: HashMap<u64, (String, u32, Option<String>)> = HashMap::new();
    if let Some(tokens) = spot_meta.get("tokens").and_then(|v| v.as_array()) {
        for tok in tokens {
            let idx = tok.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
            let name = tok
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let sz_dec = tok.get("szDecimals").and_then(|v| v.as_u64()).unwrap_or(2) as u32;
            let full_name = tok
                .get("fullName")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            token_info.insert(idx, (name, sz_dec, full_name));
        }
    }

    if let Some(universe) = spot_meta.get("universe").and_then(|v| v.as_array()) {
        for pair in universe {
            let spot_index = pair.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
            let pair_name = pair
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let is_canonical = pair
                .get("isCanonical")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let tokens_arr = pair.get("tokens").and_then(|v| v.as_array());
            let base_idx = tokens_arr
                .and_then(|a| a.first())
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            let (base_name, sz_decimals, full_name) = token_info
                .get(&base_idx)
                .cloned()
                .unwrap_or_else(|| (format!("?{}", base_idx), 2, None));

            if base_name == "USDC" || base_name.is_empty() {
                continue;
            }

            let key = format!("@{spot_index}");
            let display = if is_canonical && !pair_name.is_empty() && pair_name != key {
                Some(pair_name)
            } else {
                Some(format!("{base_name}/USDC"))
            };

            let mut kw = Vec::new();
            if let Some(fn_name) = &full_name {
                kw.push(fn_name.to_lowercase());
            }
            kw.push("spot".to_string());

            symbols.push(ExchangeSymbol {
                key,
                ticker: base_name,
                category: "spot".to_string(),
                display_name: display,
                keywords: kw,
                asset_index: 10_000 + spot_index as u32,
                sz_decimals,
                max_leverage: 1,
                only_isolated: false,
                market_type: MarketType::Spot,
                outcome: None,
            });
        }
    }
}

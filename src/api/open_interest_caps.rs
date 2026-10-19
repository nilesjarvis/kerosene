use super::{API_URL, CLIENT, proxy::HyperliquidRequestExt};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Exchange-reported cap status, independent of the displayed notional cap.
/// An empty DEX selects native perpetuals.
pub(crate) async fn fetch_perps_at_open_interest_cap(
    dex: String,
) -> Result<HashSet<String>, String> {
    if dex.contains(':') {
        return Err("Open interest cap status requires a DEX name".to_string());
    }
    let response = CLIENT
        .post(API_URL)
        .json(&serde_json::json!({ "type": "perpsAtOpenInterestCap", "dex": dex }))
        .send_info()
        .await?
        .error_for_status()
        .map_err(|e| format!("Open interest cap status HTTP error: {e}"))?
        .json::<Value>()
        .await
        .map_err(|e| format!("Open interest cap status parse failed: {e}"))?;
    parse_perps_at_open_interest_cap(&response, &dex)
}

fn parse_perps_at_open_interest_cap(
    response: &Value,
    dex: &str,
) -> Result<HashSet<String>, String> {
    let entries = response
        .as_array()
        .ok_or("Open interest cap status response must be an array")?;
    entries
        .iter()
        .map(|entry| {
            let coin = entry
                .as_str()
                .filter(|coin| !coin.is_empty())
                .ok_or("Open interest cap status contains an invalid symbol")?;
            match coin.split_once(':') {
                Some((prefix, name))
                    if !dex.is_empty()
                        && prefix == dex
                        && !name.is_empty()
                        && !name.contains(':') =>
                {
                    Ok(coin.to_string())
                }
                Some(_) => Err("Open interest cap status contains a different DEX".to_string()),
                None if dex.is_empty() => Ok(coin.to_string()),
                None => Ok(format!("{dex}:{coin}")),
            }
        })
        .collect()
}

/// Per-asset USD notional caps. The native DEX has no numeric cap endpoint.
pub(crate) async fn fetch_open_interest_caps(dex: String) -> Result<HashMap<String, f64>, String> {
    if dex.is_empty() || dex.contains(':') {
        return Err("Open interest caps require a builder DEX name".to_string());
    }
    let response = CLIENT
        .post(API_URL)
        .json(&serde_json::json!({ "type": "perpDexLimits", "dex": dex }))
        .send_info()
        .await?
        .error_for_status()
        .map_err(|e| format!("Open interest caps HTTP error: {e}"))?
        .json::<Value>()
        .await
        .map_err(|e| format!("Open interest caps parse failed: {e}"))?;
    parse_open_interest_caps(&response, &dex)
}

fn parse_open_interest_caps(response: &Value, dex: &str) -> Result<HashMap<String, f64>, String> {
    let entries = response
        .get("coinToOiCap")
        .and_then(Value::as_array)
        .ok_or("Open interest caps response missing coinToOiCap")?;
    let mut caps = HashMap::new();
    for entry in entries {
        let Some(pair) = entry.as_array().filter(|pair| pair.len() == 2) else {
            continue;
        };
        let Some(coin) = pair[0].as_str().filter(|coin| !coin.is_empty()) else {
            continue;
        };
        let cap = pair[1]
            .as_str()
            .and_then(|value| value.parse::<f64>().ok())
            .or_else(|| pair[1].as_f64())
            .filter(|cap| cap.is_finite() && *cap >= 0.0);
        let Some(cap) = cap else {
            continue;
        };
        let symbol = match coin.split_once(':') {
            Some((prefix, name)) if prefix == dex && !name.is_empty() => coin.to_string(),
            Some(_) => continue,
            None => format!("{dex}:{coin}"),
        };
        caps.insert(symbol, cap);
    }
    Ok(caps)
}

#[cfg(test)]
mod tests;

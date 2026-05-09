use serde_json::Value;
use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Spot Token Metadata
// ---------------------------------------------------------------------------

pub(in crate::account_analytics::income) fn parse_spot_token_names(
    raw: &Value,
) -> HashMap<u32, String> {
    let mut names = HashMap::new();
    let Some(tokens) = raw.get("tokens").and_then(|value| value.as_array()) else {
        return names;
    };
    for token in tokens {
        let Some(idx_u64) = token.get("index").and_then(|value| value.as_u64()) else {
            continue;
        };
        let Ok(idx) = u32::try_from(idx_u64) else {
            continue;
        };
        let name = token
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        if !name.is_empty() {
            names.insert(idx, name);
        }
    }
    names
}

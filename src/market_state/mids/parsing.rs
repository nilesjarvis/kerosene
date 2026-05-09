use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// All-Mids Parsing
// ---------------------------------------------------------------------------

pub(super) fn parse_mids_response(raw: HashMap<String, String>) -> HashMap<String, f64> {
    raw.into_iter()
        .filter_map(|(key, value)| {
            value
                .parse::<f64>()
                .ok()
                .filter(|price| price.is_finite() && *price > 0.0)
                .map(|price| (key, price))
        })
        .collect()
}

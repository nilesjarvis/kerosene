use crate::helpers::positive_finite_value;

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
                .and_then(positive_finite_value)
                .map(|price| (key, price))
        })
        .collect()
}

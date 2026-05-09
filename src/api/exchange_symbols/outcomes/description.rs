use std::collections::HashMap;

#[cfg(test)]
mod tests;

pub(super) fn parse_outcome_description(description: &str) -> HashMap<String, String> {
    description
        .split('|')
        .filter_map(|part| {
            let (key, value) = part.split_once(':')?;
            let key = key.trim();
            let value = value.trim();
            (!key.is_empty() && !value.is_empty()).then(|| (key.to_string(), value.to_string()))
        })
        .collect()
}

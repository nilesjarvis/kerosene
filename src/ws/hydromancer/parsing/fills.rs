use crate::helpers::parse_finite_number;
use serde_json::Value;

pub(in crate::ws::hydromancer) fn hydromancer_fill_items<'a>(
    json: &'a Value,
    channel: &str,
) -> Option<&'a Vec<Value>> {
    match json.get("type").and_then(|v| v.as_str()) {
        Some("replay") if json.get("channel").and_then(|v| v.as_str()) == Some(channel) => {
            json.get("data").and_then(|v| v.as_array())
        }
        Some(msg_type) if msg_type == channel => json.get("fills").and_then(|v| v.as_array()),
        _ => None,
    }
}

pub(super) fn fill_address_and_details(fill_tuple: &Value) -> Option<(String, &Value)> {
    let fill = fill_tuple.as_array()?;
    let address = fill.first()?.as_str()?.to_string();
    let details = fill.get(1)?;
    Some((address, details))
}

pub(super) fn hydromancer_str_f64(details: &Value, key: &str) -> Option<f64> {
    details
        .get(key)
        .and_then(|v| v.as_str())
        .and_then(parse_finite_number)
}

pub(super) fn hydromancer_u64(details: &Value, key: &str) -> u64 {
    details.get(key).and_then(|v| v.as_u64()).unwrap_or(0)
}

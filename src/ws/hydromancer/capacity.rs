use serde_json::Value;

/// Recognize capacity feedback before authentication redaction (limit messages
/// can mention an API key without indicating invalid credentials).
pub(super) fn is_capacity_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase().replace(['_', '-'], " ");
    lower.contains("too many subscriptions")
        || lower.contains("too many connections")
        || lower.contains("too many candles")
        || lower.contains("too many orderbooks")
        || lower.contains("subscription limit")
        || lower.contains("subscription type limit")
        || lower.contains("subscriptions limit")
        || lower.contains("connection limit")
        || lower.contains("connections limit")
        || lower.contains("max subscriptions")
        || lower.contains("maximum subscriptions")
        || lower.contains("max connections")
        || lower.contains("maximum connections")
        || lower.contains("quota exceeded")
        || lower.contains("quota exhausted")
        || lower.contains("capacity exceeded")
        || lower.contains("rate limit")
        || lower == "429"
        || (lower.contains("http") && lower.contains("429"))
        || ((lower.contains("limit") || lower.contains("maximum"))
            && (lower.contains("reached") || lower.contains("exceeded"))
            && (lower.contains("candle") || lower.contains("orderbook")))
}

pub(super) fn is_capacity_message(msg_type: &str, data: &Value) -> bool {
    if !matches!(
        msg_type,
        "error" | "disconnected" | "reconnecting" | "subscriptionUpdate" | "subscriptionResponse"
    ) || data.get("operation").and_then(Value::as_str) == Some("unsubscribe")
    {
        return false;
    }
    has_capacity_error(data)
        || data
            .get("failed")
            .and_then(Value::as_array)
            .is_some_and(|failures| failures.iter().any(has_capacity_error))
}

fn has_capacity_error(value: &Value) -> bool {
    if let Some(error) = value.as_str() {
        return is_capacity_error(error);
    }
    ["code", "message", "error"].iter().any(|field| {
        value.get(field).is_some_and(|error| {
            error.as_u64() == Some(429)
                || error.as_str().is_some_and(is_capacity_error)
                || (error.is_object() && has_capacity_error(error))
        })
    })
}

/// Only these subscriptions have native equivalents. Unknown future streams
/// remain on Hydromancer until an adapter explicitly supports fallback.
pub(super) fn has_market_fallback(payload: &Value) -> bool {
    match payload
        .pointer("/subscription/type")
        .and_then(Value::as_str)
    {
        Some("candle") => payload
            .pointer("/subscription/interval")
            .and_then(Value::as_str)
            .is_some_and(|interval| interval != "1s"),
        Some("l2Book" | "activeAssetCtx") => true,
        _ => false,
    }
}

pub(super) const MARKET_FALLBACK_MESSAGE: &str = "marketFallback";

#[cfg(test)]
mod tests;

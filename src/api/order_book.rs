use super::{API_URL, CLIENT};
use serde::Deserialize;
use serde_json::Value;

#[cfg(test)]
mod tests;

/// A single price level in the order book.
#[derive(Debug, Clone, Deserialize)]
pub struct BookLevel {
    #[serde(deserialize_with = "super::candles::de_string_or_number_to_f64")]
    pub px: f64,
    #[serde(deserialize_with = "super::candles::de_string_or_number_to_f64")]
    pub sz: f64,
}

/// Full L2 order book snapshot (bids + asks).
#[derive(Debug, Clone)]
pub struct OrderBook {
    pub bids: Vec<BookLevel>, // sorted best (highest) first
    pub asks: Vec<BookLevel>, // sorted best (lowest) first
}

impl OrderBook {
    pub fn empty() -> Self {
        Self {
            bids: Vec::new(),
            asks: Vec::new(),
        }
    }

    /// Mid price from the best bid and ask. Returns 0.0 if either side is empty.
    pub fn mid_price(&self) -> f64 {
        match (self.bids.first(), self.asks.first()) {
            (Some(bid), Some(ask)) => (bid.px + ask.px) / 2.0,
            (Some(bid), None) => bid.px,
            (None, Some(ask)) => ask.px,
            (None, None) => 0.0,
        }
    }
}

/// Fetch the L2 order book snapshot for a coin.
pub async fn fetch_order_book(
    coin: String,
    sigfigs: (Option<u8>, Option<u8>),
) -> Result<OrderBook, String> {
    let mut payload = serde_json::json!({"type": "l2Book", "coin": coin});
    let payload_object = payload
        .as_object_mut()
        .ok_or_else(|| "l2Book payload was not an object".to_string())?;
    if let Some(n) = sigfigs.0 {
        payload_object.insert("nSigFigs".to_string(), serde_json::json!(n));
    }
    if let Some(m) = sigfigs.1 {
        payload_object.insert("mantissa".to_string(), serde_json::json!(m));
    }
    let body = payload;

    let client = CLIENT.clone();
    let response = client
        .post(API_URL)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("l2Book request failed: {e}"))?;

    let raw: Value = response
        .json()
        .await
        .map_err(|e| format!("l2Book parse failed: {e}"))?;

    parse_order_book_response(&raw)
}

/// Parse an L2 book from a WebSocket JSON `data` object.
pub fn parse_ws_book(data: &Value) -> Option<OrderBook> {
    let levels = data.get("levels")?.as_array()?;
    if levels.len() < 2 {
        return None;
    }
    let bids = parse_book_side(&levels[0]).ok()?;
    let asks = parse_book_side(&levels[1]).ok()?;
    Some(OrderBook { bids, asks })
}

fn parse_book_side(value: &Value) -> Result<Vec<BookLevel>, serde_json::Error> {
    let mut levels: Vec<BookLevel> = serde_json::from_value(value.clone())?;
    levels.retain(is_valid_book_level);
    Ok(levels)
}

fn parse_order_book_response(raw: &Value) -> Result<OrderBook, String> {
    if raw.is_null() {
        return Err(
            "l2Book returned null; the symbol or aggregation parameters are unsupported"
                .to_string(),
        );
    }

    if let Some(error) = raw.get("error").and_then(Value::as_str) {
        return Err(format!("l2Book error: {error}"));
    }

    let levels = raw.get("levels").and_then(Value::as_array).ok_or_else(|| {
        format!(
            "Expected levels array in l2Book response, got {}",
            value_snippet(raw)
        )
    })?;

    if levels.len() < 2 {
        return Err("Expected [bids, asks] in levels".to_string());
    }

    let bids = parse_book_side(&levels[0]).map_err(|e| format!("Failed to parse bids: {e}"))?;
    let asks = parse_book_side(&levels[1]).map_err(|e| format!("Failed to parse asks: {e}"))?;

    Ok(OrderBook { bids, asks })
}

fn value_snippet(value: &Value) -> String {
    let rendered = value.to_string();
    let mut snippet: String = rendered.chars().take(200).collect();
    if rendered.chars().count() > 200 {
        snippet.push_str("...");
    }
    snippet
}

fn is_valid_book_level(level: &BookLevel) -> bool {
    level.px.is_finite() && level.px > 0.0 && level.sz.is_finite() && level.sz > 0.0
}

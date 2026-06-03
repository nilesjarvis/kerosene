mod model;
mod parsing;

use self::parsing::parse_order_status_inner;
use super::{API_URL, CLIENT};
use crate::helpers::text_excerpt;
pub(crate) use model::OrderStatusResult;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Order Status
// ---------------------------------------------------------------------------

pub(crate) async fn fetch_order_status_by_cloid(
    address: String,
    cloid: String,
) -> Result<OrderStatusResult, String> {
    fetch_order_status(address, serde_json::json!(cloid), Some(cloid)).await
}

pub(crate) async fn fetch_order_status_by_oid(
    address: String,
    oid: u64,
) -> Result<OrderStatusResult, String> {
    fetch_order_status(address, serde_json::json!(oid), None).await
}

async fn fetch_order_status(
    address: String,
    oid: Value,
    expected_cloid: Option<String>,
) -> Result<OrderStatusResult, String> {
    let body = serde_json::json!({
        "type": "orderStatus",
        "user": address,
        "oid": oid,
    });
    let response = CLIENT
        .clone()
        .post(API_URL)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("orderStatus request failed: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let preview = text_excerpt(&body, 160);
        return if preview.is_empty() {
            Err(format!("orderStatus request failed with HTTP {status}"))
        } else {
            Err(format!(
                "orderStatus request failed with HTTP {status}: {preview}"
            ))
        };
    }

    let raw: Value = response
        .json()
        .await
        .map_err(|e| format!("orderStatus parse failed: {e}"))?;
    parse_order_status_inner(&raw, expected_cloid.as_deref())
}

#[cfg(test)]
fn parse_order_status(raw: &Value) -> Result<OrderStatusResult, String> {
    parse_order_status_inner(raw, None)
}

#[cfg(test)]
fn parse_order_status_for_cloid(
    raw: &Value,
    expected_cloid: &str,
) -> Result<OrderStatusResult, String> {
    parse_order_status_inner(raw, Some(expected_cloid))
}

#[cfg(test)]
mod tests;

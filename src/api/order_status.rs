mod model;
mod parsing;

use self::parsing::parse_order_status_inner;
use super::{API_URL, CLIENT};
use crate::helpers::{redact_sensitive_order_text, text_excerpt};
pub(crate) use model::OrderStatusResult;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Order Status
// ---------------------------------------------------------------------------

const ORDER_STATUS_ERROR_PREVIEW_CHARS: usize = 160;

pub(crate) async fn fetch_order_status_by_cloid(
    address: String,
    cloid: String,
) -> Result<OrderStatusResult, String> {
    let result = fetch_order_status(address, serde_json::json!(cloid), None, Some(cloid)).await;
    redact_order_status_result(result)
}

pub(crate) async fn fetch_order_status_by_oid(
    address: String,
    oid: u64,
) -> Result<OrderStatusResult, String> {
    let result = fetch_order_status(address, serde_json::json!(oid), Some(oid), None).await;
    redact_order_status_result(result)
}

fn redact_order_status_result(
    result: Result<OrderStatusResult, String>,
) -> Result<OrderStatusResult, String> {
    result.map_err(|error| redact_sensitive_order_text(&error))
}

async fn fetch_order_status(
    address: String,
    oid: Value,
    expected_oid: Option<u64>,
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
        let preview = order_status_error_preview(&body);
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
    parse_order_status_inner(&raw, expected_oid, expected_cloid.as_deref())
}

fn order_status_error_preview(body: &str) -> String {
    text_excerpt(
        &redact_sensitive_order_text(body),
        ORDER_STATUS_ERROR_PREVIEW_CHARS,
    )
}

#[cfg(test)]
fn parse_order_status(raw: &Value) -> Result<OrderStatusResult, String> {
    parse_order_status_inner(raw, None, None)
}

#[cfg(test)]
fn parse_order_status_for_oid(raw: &Value, expected_oid: u64) -> Result<OrderStatusResult, String> {
    parse_order_status_inner(raw, Some(expected_oid), None)
}

#[cfg(test)]
fn parse_order_status_for_cloid(
    raw: &Value,
    expected_cloid: &str,
) -> Result<OrderStatusResult, String> {
    parse_order_status_inner(raw, None, Some(expected_cloid))
}

#[cfg(test)]
mod tests;

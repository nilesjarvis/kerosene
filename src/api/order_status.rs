use super::{API_URL, CLIENT};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Order Status
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OrderStatusResult {
    pub(crate) status: String,
    pub(crate) oid: Option<u64>,
    pub(crate) cloid: Option<String>,
    pub(crate) raw_summary: String,
}

impl OrderStatusResult {
    pub(crate) fn is_missing(&self) -> bool {
        let status = self.status.to_ascii_lowercase();
        status.contains("unknown") || status.contains("missing")
    }

    pub(crate) fn is_open(&self) -> bool {
        self.status.eq_ignore_ascii_case("open")
    }

    pub(crate) fn is_filled(&self) -> bool {
        self.status.eq_ignore_ascii_case("filled")
    }

    pub(crate) fn is_no_fill_terminal(&self) -> bool {
        let status = self.status.to_ascii_lowercase();
        matches!(
            status.as_str(),
            "canceled"
                | "cancelled"
                | "rejected"
                | "ioccancelrejected"
                | "mintradentlrejected"
                | "tickrejected"
                | "reduceonlyrejected"
                | "reduceonlycanceled"
                | "selftradecanceled"
                | "scheduledcancel"
                | "margincanceled"
                | "perpmarginrejected"
                | "insufficientspotbalancerejected"
                | "oraclejected"
                | "oraclerejected"
                | "openinterestcapcanceled"
                | "positionincreaseatopeninterestcaprejected"
                | "positionflipatopeninterestcaprejected"
                | "tooaggressiveatopeninterestcaprejected"
                | "openinterestincreaserejected"
                | "perpmaxpositionrejected"
                | "delistedcanceled"
                | "liquidatedcanceled"
        )
    }
}

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
        let preview = body.chars().take(160).collect::<String>();
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

fn parse_order_status_inner(
    raw: &Value,
    expected_cloid: Option<&str>,
) -> Result<OrderStatusResult, String> {
    if let Some(error) = raw.get("error").and_then(Value::as_str) {
        return Err(format!("orderStatus error: {error}"));
    }

    if raw.get("status").and_then(Value::as_str) == Some("order") {
        let order = raw
            .get("order")
            .ok_or_else(|| "orderStatus response missing order".to_string())?;
        let status = order
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let order_body = order.get("order");
        let oid = order_body
            .and_then(|value| value.get("oid"))
            .and_then(Value::as_u64);
        let cloid = order_body
            .and_then(|value| value.get("cloid"))
            .and_then(Value::as_str)
            .map(ToString::to_string);
        if let Some(expected_cloid) = expected_cloid
            && cloid.as_deref() != Some(expected_cloid)
        {
            return Err(format!(
                "orderStatus response cloid mismatch for {expected_cloid}: got {}",
                cloid.as_deref().unwrap_or("missing cloid")
            ));
        }
        return Ok(OrderStatusResult {
            raw_summary: format_order_status_summary(&status, oid, cloid.as_deref()),
            status,
            oid,
            cloid,
        });
    }

    let status = raw
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    Ok(OrderStatusResult {
        raw_summary: status.clone(),
        status,
        oid: None,
        cloid: None,
    })
}

fn format_order_status_summary(status: &str, oid: Option<u64>, cloid: Option<&str>) -> String {
    match (oid, cloid) {
        (Some(oid), Some(cloid)) => format!("{status} (oid {oid}, cloid {cloid})"),
        (Some(oid), None) => format!("{status} (oid {oid})"),
        (None, Some(cloid)) => format!("{status} (cloid {cloid})"),
        (None, None) => status.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_order_status, parse_order_status_for_cloid};

    #[test]
    fn parses_order_status_by_cloid_response() {
        let parsed = parse_order_status(&serde_json::json!({
            "status": "order",
            "order": {
                "status": "open",
                "order": {
                    "oid": 42_u64,
                    "cloid": "0x1234567890abcdef1234567890abcdef"
                }
            }
        }))
        .expect("order status should parse");

        assert!(parsed.is_open());
        assert_eq!(parsed.oid, Some(42));
        assert_eq!(
            parsed.cloid.as_deref(),
            Some("0x1234567890abcdef1234567890abcdef")
        );
    }

    #[test]
    fn parses_missing_order_status() {
        let parsed = parse_order_status(&serde_json::json!({
            "status": "unknownOid"
        }))
        .expect("missing status should parse");

        assert!(parsed.is_missing());
    }

    #[test]
    fn rejects_mismatched_order_status_cloid() {
        let error = parse_order_status_for_cloid(
            &serde_json::json!({
                "status": "order",
                "order": {
                    "status": "open",
                    "order": {
                        "oid": 42_u64,
                        "cloid": "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    }
                }
            }),
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .expect_err("mismatched cloid should be rejected");

        assert!(error.contains("cloid mismatch"));
    }

    #[test]
    fn rejects_order_status_without_expected_cloid() {
        let error = parse_order_status_for_cloid(
            &serde_json::json!({
                "status": "order",
                "order": {
                    "status": "open",
                    "order": {
                        "oid": 42_u64
                    }
                }
            }),
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .expect_err("missing cloid should be rejected");

        assert!(error.contains("missing cloid"));
    }
}

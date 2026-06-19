use super::model::OrderStatusResult;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Order Status Parsing
// ---------------------------------------------------------------------------

pub(super) fn parse_order_status_inner(
    raw: &Value,
    expected_oid: Option<u64>,
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
        if let Some(expected_oid) = expected_oid
            && oid != Some(expected_oid)
        {
            return Err(format!(
                "orderStatus response oid mismatch for {expected_oid}: got {}",
                oid.map(|oid| oid.to_string())
                    .unwrap_or_else(|| "missing oid".to_string())
            ));
        }
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

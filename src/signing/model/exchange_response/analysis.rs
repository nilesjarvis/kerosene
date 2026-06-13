use crate::helpers::{positive_finite_value, redact_sensitive_response_text};

use super::ExchangeResponse;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Exchange Response Analysis
// ---------------------------------------------------------------------------

impl ExchangeResponse {
    /// Extract a human-readable summary from the response.
    pub fn summary(&self) -> String {
        if self.status != "ok" {
            if let Some(raw) = &self.raw_response {
                return format!("Error: {}", raw_exchange_response_summary(raw));
            }
            return format!(
                "Error: status={}",
                redact_sensitive_response_text(&self.status)
            );
        }
        let Some(inner) = &self.response else {
            return "No response body".to_string();
        };
        let Some(data) = &inner.data else {
            return format!("OK ({})", inner.response_type);
        };
        if data.statuses.is_empty() {
            return "OK (no statuses)".to_string();
        }
        let response_type = inner.response_type.as_str();
        data.statuses
            .iter()
            .map(|status| status_summary(status, response_type))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Extract the order OID from the response.
    pub fn order_oid(&self) -> Option<u64> {
        let st = self.response.as_ref()?.data.as_ref()?.statuses.first()?;
        if let Some(resting) = st.get("resting")
            && let Some(oid) = resting.get("oid").and_then(|v| v.as_u64())
        {
            return Some(oid);
        }
        if let Some(filled) = st.get("filled")
            && let Some(oid) = filled.get("oid").and_then(|v| v.as_u64())
        {
            return Some(oid);
        }
        None
    }

    /// Whether the order was immediately and fully filled.
    pub fn is_fully_filled(&self) -> bool {
        let Some(statuses) = self
            .response
            .as_ref()
            .and_then(|r| r.data.as_ref())
            .map(|d| d.statuses.as_slice())
        else {
            return false;
        };
        !statuses.is_empty()
            && statuses
                .iter()
                .all(|st| st.get("filled").is_some() && st.get("resting").is_none())
            && !self.is_error()
    }

    pub fn filled_total_size(&self) -> Option<f64> {
        let statuses = self
            .response
            .as_ref()
            .and_then(|r| r.data.as_ref())
            .map(|d| d.statuses.as_slice())?;
        let total = statuses.iter().filter_map(filled_status_size).sum();
        positive_finite_value(total)
    }

    /// Whether the response indicates an error.
    pub fn is_error(&self) -> bool {
        if self.status != "ok" {
            return true;
        }
        if let Some(inner) = &self.response
            && let Some(data) = &inner.data
            && !data.statuses.is_empty()
        {
            return data
                .statuses
                .iter()
                .any(|status| status.get("error").is_some());
        }
        false
    }

    /// Whether an order placement response is too ambiguous to continue automation safely.
    pub fn is_ambiguous_order_result(&self) -> bool {
        if self.status != "ok" {
            return false;
        }
        if self.raw_response.is_some() {
            return true;
        }
        let Some(statuses) = self
            .response
            .as_ref()
            .and_then(|inner| inner.data.as_ref())
            .map(|data| data.statuses.as_slice())
        else {
            return true;
        };
        if statuses.is_empty() {
            return true;
        }
        statuses.iter().any(ambiguous_order_status)
    }

    /// Whether a cancel response explicitly confirms cancellation.
    pub fn is_confirmed_cancel_result(&self) -> bool {
        if self.status != "ok" || self.raw_response.is_some() {
            return false;
        }
        let Some(inner) = &self.response else {
            return false;
        };
        if inner.response_type != "cancel" {
            return false;
        }
        let Some(data) = &inner.data else {
            return false;
        };
        !data.statuses.is_empty()
            && data
                .statuses
                .iter()
                .all(|status| status.as_str() == Some("success"))
    }

    /// Whether a modify response explicitly confirms the replacement order state.
    pub fn is_confirmed_modify_result(&self) -> bool {
        if self.status != "ok" || self.raw_response.is_some() || self.is_error() {
            return false;
        }
        let Some(inner) = &self.response else {
            return false;
        };
        if inner.response_type != "order" {
            return false;
        }
        let Some(data) = &inner.data else {
            return false;
        };
        !data.statuses.is_empty() && data.statuses.iter().all(confirmed_modify_status)
    }

    /// Whether a default exchange response explicitly confirms a non-order mutation.
    pub fn is_confirmed_default_result(&self) -> bool {
        self.status == "ok"
            && self.raw_response.is_none()
            && self
                .response
                .as_ref()
                .is_some_and(|inner| inner.response_type == "default" && inner.data.is_none())
    }

    /// Hyperliquid reports this for IOC orders that were marketable from the
    /// client's last book snapshot but found no resting liquidity at match time.
    pub fn is_ioc_no_match(&self) -> bool {
        self.error_messages().iter().any(|message| {
            message
                .to_ascii_lowercase()
                .contains("could not immediately match against any resting orders")
        })
    }

    fn error_messages(&self) -> Vec<String> {
        let mut messages = Vec::new();
        if self.status != "ok" {
            messages.push(self.status.clone());
        }
        if let Some(raw) = &self.raw_response {
            messages.push(raw_exchange_response_summary(raw));
        }
        if let Some(inner) = &self.response
            && let Some(data) = &inner.data
        {
            messages.extend(data.statuses.iter().filter_map(|status| {
                status
                    .get("error")
                    .and_then(|error| error.as_str())
                    .map(ToString::to_string)
            }));
        }
        messages
    }
}

fn ambiguous_order_status(status: &Value) -> bool {
    if status.get("error").is_some() {
        return false;
    }
    if let Some(resting) = status.get("resting") {
        return resting
            .get("oid")
            .and_then(|value| value.as_u64())
            .is_none();
    }
    if status.get("filled").is_some() {
        return filled_status_size(status).is_none();
    }
    true
}

fn confirmed_modify_status(status: &Value) -> bool {
    if status.as_str() == Some("success") {
        return true;
    }
    if let Some(resting) = status.get("resting") {
        return resting
            .get("oid")
            .and_then(|value| value.as_u64())
            .is_some();
    }
    if let Some(filled) = status.get("filled") {
        return filled.get("oid").and_then(|value| value.as_u64()).is_some()
            && filled_status_size(status).is_some();
    }
    false
}

fn filled_status_size(status: &Value) -> Option<f64> {
    status
        .get("filled")?
        .get("totalSz")
        .and_then(|value| value.as_str())
        .and_then(|value| value.parse::<f64>().ok())
        .and_then(positive_finite_value)
}

fn raw_exchange_response_summary(value: &Value) -> String {
    let summary = value
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_else(|| value.to_string());
    redact_sensitive_response_text(&summary)
}

fn status_summary(st: &Value, response_type: &str) -> String {
    if let Some(err) = st.get("error").and_then(|v| v.as_str()) {
        return format!("Error: {}", redact_sensitive_response_text(err));
    }
    if let Some(filled) = st.get("filled") {
        let sz = filled
            .get("totalSz")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let px = filled.get("avgPx").and_then(|v| v.as_str()).unwrap_or("?");
        let oid = filled.get("oid").and_then(|v| v.as_u64()).unwrap_or(0);
        return format!("Filled {sz} @ ${px} (oid {oid})");
    }
    if let Some(resting) = st.get("resting") {
        let oid = resting.get("oid").and_then(|v| v.as_u64()).unwrap_or(0);
        return format!("Resting (oid {oid})");
    }
    if st.as_str() == Some("success") {
        // A bare "success" status only means "cancelled" for cancel actions;
        // modify/order acks of the same shape must not report a cancel (the
        // result classifier keys ExecutionOutcomeKind::Cancelled on this
        // string).
        return if response_type == "cancel" {
            "Cancelled".to_string()
        } else {
            "Success".to_string()
        };
    }
    redact_sensitive_response_text(&format!("{st}"))
}

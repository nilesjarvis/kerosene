use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::fmt;

mod analysis;

// ---------------------------------------------------------------------------
// Exchange Response Model
// ---------------------------------------------------------------------------

/// Response from the exchange API.
#[derive(Clone)]
pub struct ExchangeResponse {
    pub status: String,
    pub response: Option<ExchangeResponseInner>,
    raw_response: Option<Value>,
}

#[derive(Clone, Deserialize)]
pub struct ExchangeResponseInner {
    #[serde(rename = "type")]
    pub response_type: String,
    pub data: Option<ExchangeResponseData>,
}

impl ExchangeResponseInner {
    fn redacted_response_type(&self) -> &str {
        match self.response_type.as_str() {
            "order" | "cancel" | "default" => self.response_type.as_str(),
            _ => "<redacted>",
        }
    }
}

impl fmt::Debug for ExchangeResponseInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status_count = self.data.as_ref().map(|data| data.statuses.len());

        f.debug_struct("ExchangeResponseInner")
            .field("response_type", &self.redacted_response_type())
            .field("data", &self.data.as_ref().map(|_| "<redacted>"))
            .field("status_count", &status_count)
            .finish()
    }
}

#[derive(Clone, Deserialize)]
pub struct ExchangeResponseData {
    pub statuses: Vec<Value>,
}

impl fmt::Debug for ExchangeResponseData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExchangeResponseData")
            .field(
                "statuses",
                &format_args!("<redacted>; count={}", self.statuses.len()),
            )
            .finish()
    }
}

#[derive(Deserialize)]
struct ExchangeResponseWire {
    status: String,
    response: Option<Value>,
}

impl<'de> Deserialize<'de> for ExchangeResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ExchangeResponseWire::deserialize(deserializer)?;
        let mut response = None;
        let mut raw_response = None;

        if let Some(raw) = wire.response {
            match serde_json::from_value::<ExchangeResponseInner>(raw.clone()) {
                Ok(inner) => response = Some(inner),
                Err(_) => raw_response = Some(raw),
            }
        }

        Ok(Self {
            status: wire.status,
            response,
            raw_response,
        })
    }
}

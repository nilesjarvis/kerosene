use serde::{Deserialize, Deserializer};
use serde_json::Value;

mod analysis;

// ---------------------------------------------------------------------------
// Exchange Response Model
// ---------------------------------------------------------------------------

/// Response from the exchange API.
#[derive(Debug, Clone)]
pub struct ExchangeResponse {
    pub status: String,
    pub response: Option<ExchangeResponseInner>,
    raw_response: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExchangeResponseInner {
    #[serde(rename = "type")]
    pub response_type: String,
    pub data: Option<ExchangeResponseData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExchangeResponseData {
    pub statuses: Vec<Value>,
}

#[derive(Debug, Deserialize)]
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

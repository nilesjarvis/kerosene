use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// User Fill Models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFill {
    pub coin: String,
    pub px: String,
    pub sz: String,
    pub side: String,
    pub time: u64,
    #[serde(rename = "startPosition")]
    pub start_position: String,
    pub dir: String,
    #[serde(rename = "closedPnl")]
    pub closed_pnl: String,
    pub hash: String,
    pub oid: u64,
    pub crossed: bool,
    pub fee: String,
    pub tid: u64,
    #[serde(rename = "feeToken")]
    pub fee_token: String,
}

#[derive(Debug, Clone, Copy)]
pub struct UserFillsRequest {
    pub start_time: u64,
    pub end_time: Option<u64>,
}

impl UserFillsRequest {
    pub fn full_history() -> Self {
        Self {
            start_time: 0,
            end_time: None,
        }
    }

    pub fn since(last_seen_time: Option<u64>) -> Self {
        Self {
            start_time: last_seen_time.unwrap_or(0),
            end_time: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserFillsPage {
    pub fills: Vec<UserFill>,
    pub next_request: Option<UserFillsRequest>,
    pub requested_end_time: u64,
    pub progress_warning: Option<String>,
}

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// User Fill Models
// ---------------------------------------------------------------------------

#[derive(Clone, Serialize, Deserialize)]
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

impl fmt::Debug for UserFill {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("UserFill").field(&"<redacted>").finish()
    }
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

#[derive(Clone)]
pub struct UserFillsPage {
    pub fills: Vec<UserFill>,
    pub next_request: Option<UserFillsRequest>,
    pub requested_end_time: u64,
    pub progress_warning: Option<String>,
}

impl fmt::Debug for UserFillsPage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserFillsPage")
            .field("fills", &format_args!("len={}", self.fills.len()))
            .field("next_request", &self.next_request)
            .field("requested_end_time", &self.requested_end_time)
            .field(
                "progress_warning",
                &self.progress_warning.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{UserFill, UserFillsPage, UserFillsRequest};

    #[test]
    fn user_fill_debug_redacts_trade_payload() {
        let fill = fill();

        let rendered = format!("{fill:?}");

        assert!(rendered.contains("<redacted>"));
        for secret in [
            "SECRETCOIN",
            "fill-secret-price",
            "fill-secret-size",
            "fill-secret-start-position",
            "fill-secret-pnl",
            "fill-secret-hash",
            "fill-secret-fee",
        ] {
            assert!(!rendered.contains(secret), "{secret} leaked in {rendered}");
        }
    }

    #[test]
    fn user_fills_page_debug_summarizes_fills_and_warning() {
        let page = UserFillsPage {
            fills: vec![fill()],
            next_request: Some(UserFillsRequest {
                start_time: 10,
                end_time: Some(20),
            }),
            requested_end_time: 30,
            progress_warning: Some("fills-warning-secret".to_string()),
        };

        let rendered = format!("{page:?}");

        assert!(rendered.contains("fills: len=1"));
        assert!(rendered.contains("start_time: 10"));
        assert!(rendered.contains("requested_end_time: 30"));
        assert!(rendered.contains("progress_warning: Some(\"<redacted>\")"));
        for secret in [
            "SECRETCOIN",
            "fill-secret-price",
            "fill-secret-hash",
            "fills-warning-secret",
        ] {
            assert!(!rendered.contains(secret), "{secret} leaked in {rendered}");
        }
    }

    fn fill() -> UserFill {
        UserFill {
            coin: "SECRETCOIN".to_string(),
            px: "fill-secret-price".to_string(),
            sz: "fill-secret-size".to_string(),
            side: "B".to_string(),
            time: 1,
            start_position: "fill-secret-start-position".to_string(),
            dir: "Open Long".to_string(),
            closed_pnl: "fill-secret-pnl".to_string(),
            hash: "fill-secret-hash".to_string(),
            oid: 42,
            crossed: false,
            fee: "fill-secret-fee".to_string(),
            tid: 7,
            fee_token: "USDC".to_string(),
        }
    }
}

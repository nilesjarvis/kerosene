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

#[derive(Clone, Copy)]
pub struct UserFillsRequest {
    pub start_time: u64,
    pub end_time: Option<u64>,
}

impl fmt::Debug for UserFillsRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserFillsRequest")
            .field("start_time", &"<redacted>")
            .field("has_end_time", &self.end_time.is_some())
            .finish()
    }
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
            .field("has_next_request", &self.next_request.is_some())
            .field("requested_end_time", &"<redacted>")
            .field(
                "has_progress_warning",
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
    fn fill_pagination_debug_redacts_account_timing_without_changing_it() {
        let page = UserFillsPage {
            fills: vec![],
            next_request: Some(UserFillsRequest {
                start_time: 9_876_543_210,
                end_time: Some(9_876_543_211),
            }),
            requested_end_time: 9_876_543_212,
            progress_warning: Some("private-fill-page-warning-sentinel".to_string()),
        };

        let rendered = format!("{page:?} {:?}", page.next_request);

        assert!(rendered.contains("<redacted>"), "{rendered}");
        assert!(rendered.contains("has_next_request: true"), "{rendered}");
        assert!(!rendered.contains("9876543210"), "{rendered}");
        assert!(!rendered.contains("9876543211"), "{rendered}");
        assert!(!rendered.contains("9876543212"), "{rendered}");
        assert!(
            !rendered.contains("private-fill-page-warning-sentinel"),
            "{rendered}"
        );
        assert_eq!(
            page.next_request
                .map(|request| (request.start_time, request.end_time)),
            Some((9_876_543_210, Some(9_876_543_211)))
        );
        assert_eq!(page.requested_end_time, 9_876_543_212);
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
        assert!(rendered.contains("has_next_request: true"));
        assert!(rendered.contains("requested_end_time: \"<redacted>\""));
        assert!(rendered.contains("has_progress_warning: Some(\"<redacted>\")"));
        for secret in [
            "SECRETCOIN",
            "fill-secret-price",
            "fill-secret-hash",
            "fills-warning-secret",
            "start_time: 10",
            "requested_end_time: 30",
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

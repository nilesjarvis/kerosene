use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Serialize, Deserialize)]
pub struct WatchlistContext {
    pub funding: Option<f64>,
    pub prev_day_px: Option<f64>,
    pub day_vlm: Option<f64>,
}

impl fmt::Debug for WatchlistContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WatchlistContext")
            .field("has_funding", &self.funding.is_some())
            .field("has_prev_day_px", &self.prev_day_px.is_some())
            .field("has_day_vlm", &self.day_vlm.is_some())
            .finish()
    }
}

/// A scoped context refresh. Healthy market families are returned even when
/// another requested family fails; `partial_errors` lets UI surfaces report
/// the degraded refresh without discarding those healthy values.
#[derive(Clone)]
pub struct WatchlistContextsResponse {
    pub contexts: HashMap<String, WatchlistContext>,
    pub partial_errors: Vec<String>,
}

impl WatchlistContextsResponse {
    pub fn complete(contexts: HashMap<String, WatchlistContext>) -> Self {
        Self {
            contexts,
            partial_errors: Vec::new(),
        }
    }
}

impl From<HashMap<String, WatchlistContext>> for WatchlistContextsResponse {
    fn from(contexts: HashMap<String, WatchlistContext>) -> Self {
        Self::complete(contexts)
    }
}

impl fmt::Debug for WatchlistContextsResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WatchlistContextsResponse")
            .field("contexts_len", &self.contexts.len())
            .field("partial_error_count", &self.partial_errors.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{WatchlistContext, WatchlistContextsResponse};
    use std::collections::HashMap;

    #[test]
    fn watchlist_context_debug_redacts_market_payload() {
        let secrets = ["0.000987", "2718.28", "314159.26"];
        let context = WatchlistContext {
            funding: Some(0.000987),
            prev_day_px: Some(2718.28),
            day_vlm: Some(314159.26),
        };

        let rendered = format!("{context:?}");

        assert!(rendered.contains("has_funding: true"));
        assert!(rendered.contains("has_prev_day_px: true"));
        assert!(rendered.contains("has_day_vlm: true"));
        for secret in secrets {
            assert!(
                !rendered.contains(secret),
                "watchlist context Debug leaked {secret}"
            );
        }
    }

    #[test]
    fn watchlist_response_debug_redacts_partial_error_details() {
        let response = WatchlistContextsResponse {
            contexts: HashMap::new(),
            partial_errors: vec!["api_key=secret-sentinel".to_string()],
        };

        let rendered = format!("{response:?}");

        assert!(rendered.contains("partial_error_count: 1"));
        assert!(!rendered.contains("secret-sentinel"));
    }
}

use std::fmt;

#[derive(Clone)]
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

#[cfg(test)]
mod tests {
    use super::WatchlistContext;

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
}

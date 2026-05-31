use crate::hyperdash_api::{LiquidationBucket, LiquidationLevel, bucket_liquidations};

use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Liquidations Distribution State
// ---------------------------------------------------------------------------

pub(crate) const LIQUIDATION_DISTRIBUTION_BUCKET_COUNT: usize = 240;
pub(crate) const LIQUIDATION_DISTRIBUTION_AUTO_REFRESH_SECS: u64 = 60;
pub(crate) const LIQUIDATION_DISTRIBUTION_REQUEST_BACKOFF_SECS: u64 = 5;
pub(crate) const LIQUIDATION_DISTRIBUTION_MARK_REFRESH_THRESHOLD: f64 = 0.01;

#[derive(Debug, Clone, Default)]
pub(crate) struct LiquidationDistributionState {
    pub(crate) data: Option<LiquidationDistributionData>,
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
    pub(crate) pending_request: Option<LiquidationDistributionRequest>,
    pub(crate) last_request: Option<Instant>,
    pub(crate) last_request_symbol: Option<String>,
    pub(crate) last_fetch: Option<Instant>,
}

impl LiquidationDistributionState {
    pub(crate) fn data_matches_symbol(&self, symbol: &str) -> bool {
        self.data
            .as_ref()
            .is_some_and(|data| data.request.symbol == symbol)
    }

    pub(crate) fn clear_data_if_not_symbol(&mut self, symbol: &str) {
        if !self.data_matches_symbol(symbol) {
            self.data = None;
            self.last_fetch = None;
        }
    }

    pub(crate) fn should_fetch(
        &self,
        request: &LiquidationDistributionRequest,
        force: bool,
    ) -> bool {
        if force {
            return true;
        }
        if self
            .last_request
            .is_some_and(|last_request| last_request.elapsed() < request_backoff())
            && self.last_request_symbol.as_deref() == Some(request.symbol.as_str())
        {
            return false;
        }
        let Some(data) = &self.data else {
            return true;
        };
        if data.request.symbol != request.symbol {
            return true;
        }
        let Some(last_fetch) = self.last_fetch else {
            return true;
        };
        if last_fetch.elapsed() >= auto_refresh_interval() {
            return true;
        }
        mark_drift_exceeds_threshold(data.request.mark, request.mark)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LiquidationDistributionRequest {
    pub(crate) key: String,
    pub(crate) symbol: String,
    pub(crate) display: String,
    pub(crate) coin: String,
    pub(crate) mark: f64,
    pub(crate) min: f64,
    pub(crate) max: f64,
    pub(crate) timestamp_secs: u64,
}

impl LiquidationDistributionRequest {
    pub(crate) fn new(
        symbol: String,
        display: String,
        coin: String,
        mark: f64,
        min: f64,
        max: f64,
        timestamp_secs: u64,
    ) -> Self {
        Self {
            key: liquidation_distribution_request_key(&coin, min, max, timestamp_secs),
            symbol,
            display,
            coin,
            mark,
            min,
            max,
            timestamp_secs,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LiquidationDistributionData {
    pub(crate) request: LiquidationDistributionRequest,
    pub(crate) points: Vec<LiquidationDistributionPoint>,
    pub(crate) raw_count: usize,
    pub(crate) total_long_usd: f64,
    pub(crate) total_short_usd: f64,
    pub(crate) max_bucket_usd: f64,
    pub(crate) max_cumulative_usd: f64,
    pub(crate) fetched_at_ms: u64,
}

impl LiquidationDistributionData {
    pub(crate) fn from_level(
        request: LiquidationDistributionRequest,
        level: LiquidationLevel,
        fetched_at_ms: u64,
    ) -> Self {
        let buckets = bucket_liquidations(
            &level.liquidations,
            level.min,
            level.max,
            LIQUIDATION_DISTRIBUTION_BUCKET_COUNT,
        );
        let points = distribution_points_from_buckets(&buckets, request.mark);
        let raw_count = level.liquidations.len();
        let total_long_usd = points.iter().map(|point| point.long_usd).sum();
        let total_short_usd = points.iter().map(|point| point.short_usd).sum();
        let max_bucket_usd = points
            .iter()
            .map(|point| point.long_usd.max(point.short_usd))
            .fold(0.0, f64::max);
        let max_cumulative_usd = points
            .iter()
            .map(|point| point.cumulative_long_usd.max(point.cumulative_short_usd))
            .fold(0.0, f64::max);

        Self {
            request,
            points,
            raw_count,
            total_long_usd,
            total_short_usd,
            max_bucket_usd,
            max_cumulative_usd,
            fetched_at_ms,
        }
    }

    pub(crate) fn has_values(&self) -> bool {
        self.max_bucket_usd > 0.0 || self.max_cumulative_usd > 0.0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct LiquidationDistributionPoint {
    pub(crate) price: f64,
    pub(crate) long_usd: f64,
    pub(crate) short_usd: f64,
    pub(crate) cumulative_long_usd: f64,
    pub(crate) cumulative_short_usd: f64,
}

pub(crate) fn liquidation_distribution_request_key(
    coin: &str,
    min: f64,
    max: f64,
    timestamp_secs: u64,
) -> String {
    format!("{coin}:{min:.8}:{max:.8}:{timestamp_secs}")
}

pub(crate) fn validate_liquidation_distribution_level(
    request: &LiquidationDistributionRequest,
    level: &LiquidationLevel,
) -> Result<(), String> {
    if level.coin != request.coin {
        return Err(format!(
            "HyperDash returned {} data for {} request",
            level.coin, request.coin
        ));
    }
    validate_price_bound("min", request.min, level.min)?;
    validate_price_bound("max", request.max, level.max)?;
    Ok(())
}

pub(crate) fn distribution_points_from_buckets(
    buckets: &[LiquidationBucket],
    mark: f64,
) -> Vec<LiquidationDistributionPoint> {
    let mut points: Vec<LiquidationDistributionPoint> = buckets
        .iter()
        .map(|bucket| LiquidationDistributionPoint {
            price: bucket.price_center,
            long_usd: bucket.long_usd,
            short_usd: bucket.short_usd,
            cumulative_long_usd: 0.0,
            cumulative_short_usd: 0.0,
        })
        .collect();

    let mut cumulative_longs = 0.0;
    for point in points.iter_mut().rev() {
        if point.price <= mark {
            cumulative_longs += point.long_usd;
            point.cumulative_long_usd = cumulative_longs;
        }
    }

    let mut cumulative_shorts = 0.0;
    for point in &mut points {
        if point.price >= mark {
            cumulative_shorts += point.short_usd;
            point.cumulative_short_usd = cumulative_shorts;
        }
    }

    points
}

fn auto_refresh_interval() -> Duration {
    Duration::from_secs(LIQUIDATION_DISTRIBUTION_AUTO_REFRESH_SECS)
}

fn request_backoff() -> Duration {
    Duration::from_secs(LIQUIDATION_DISTRIBUTION_REQUEST_BACKOFF_SECS)
}

fn mark_drift_exceeds_threshold(previous: f64, current: f64) -> bool {
    if !previous.is_finite() || !current.is_finite() || previous <= 0.0 {
        return true;
    }
    ((current - previous).abs() / previous) >= LIQUIDATION_DISTRIBUTION_MARK_REFRESH_THRESHOLD
}

fn validate_price_bound(name: &str, expected: f64, actual: f64) -> Result<(), String> {
    if !expected.is_finite() || !actual.is_finite() {
        return Err(format!(
            "HyperDash returned non-finite {name} bound for liquidation distribution"
        ));
    }
    let tolerance = expected.abs().max(actual.abs()).max(1.0) * 1e-6;
    if (expected - actual).abs() > tolerance {
        return Err(format!(
            "HyperDash returned {name} bound {actual:.8}, expected {expected:.8}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bucket(price_center: f64, long_usd: f64, short_usd: f64) -> LiquidationBucket {
        LiquidationBucket {
            price_center,
            long_coins: 0.0,
            short_coins: 0.0,
            long_usd,
            short_usd,
        }
    }

    fn request(symbol: &str, mark: f64) -> LiquidationDistributionRequest {
        LiquidationDistributionRequest::new(
            symbol.to_string(),
            symbol.to_string(),
            symbol.to_string(),
            mark,
            0.0,
            200.0,
            100,
        )
    }

    fn data(symbol: &str, mark: f64) -> LiquidationDistributionData {
        LiquidationDistributionData {
            request: request(symbol, mark),
            points: Vec::new(),
            raw_count: 0,
            total_long_usd: 0.0,
            total_short_usd: 0.0,
            max_bucket_usd: 0.0,
            max_cumulative_usd: 0.0,
            fetched_at_ms: 0,
        }
    }

    #[test]
    fn distribution_points_accumulate_longs_below_mark_from_current_down() {
        let buckets = vec![
            bucket(80.0, 10.0, 0.0),
            bucket(90.0, 20.0, 0.0),
            bucket(100.0, 30.0, 0.0),
            bucket(110.0, 40.0, 0.0),
        ];

        let points = distribution_points_from_buckets(&buckets, 100.0);

        assert_eq!(points[0].cumulative_long_usd, 60.0);
        assert_eq!(points[1].cumulative_long_usd, 50.0);
        assert_eq!(points[2].cumulative_long_usd, 30.0);
        assert_eq!(points[3].cumulative_long_usd, 0.0);
    }

    #[test]
    fn distribution_points_accumulate_shorts_above_mark_from_current_up() {
        let buckets = vec![
            bucket(80.0, 0.0, 10.0),
            bucket(90.0, 0.0, 20.0),
            bucket(100.0, 0.0, 30.0),
            bucket(110.0, 0.0, 40.0),
        ];

        let points = distribution_points_from_buckets(&buckets, 100.0);

        assert_eq!(points[0].cumulative_short_usd, 0.0);
        assert_eq!(points[1].cumulative_short_usd, 0.0);
        assert_eq!(points[2].cumulative_short_usd, 30.0);
        assert_eq!(points[3].cumulative_short_usd, 70.0);
    }

    #[test]
    fn clears_data_when_symbol_no_longer_matches() {
        let mut state = LiquidationDistributionState {
            data: Some(data("BTC", 100.0)),
            last_fetch: Some(Instant::now()),
            ..Default::default()
        };

        state.clear_data_if_not_symbol("ETH");

        assert!(state.data.is_none());
        assert!(state.last_fetch.is_none());
    }

    #[test]
    fn should_fetch_skips_recent_same_symbol_snapshot() {
        let state = LiquidationDistributionState {
            data: Some(data("BTC", 100.0)),
            last_fetch: Some(Instant::now()),
            ..Default::default()
        };

        assert!(!state.should_fetch(&request("BTC", 100.5), false));
    }

    #[test]
    fn should_fetch_when_recent_mark_drift_exceeds_threshold() {
        let state = LiquidationDistributionState {
            data: Some(data("BTC", 100.0)),
            last_fetch: Some(Instant::now()),
            ..Default::default()
        };

        assert!(state.should_fetch(&request("BTC", 102.0), false));
    }

    #[test]
    fn validation_rejects_coin_mismatch() {
        let level = LiquidationLevel {
            coin: "ETH".to_string(),
            min: 0.0,
            max: 200.0,
            liquidations: Vec::new(),
        };

        let result = validate_liquidation_distribution_level(&request("BTC", 100.0), &level);

        assert!(result.is_err());
    }

    #[test]
    fn validation_rejects_range_mismatch() {
        let level = LiquidationLevel {
            coin: "BTC".to_string(),
            min: 0.0,
            max: 201.0,
            liquidations: Vec::new(),
        };

        let result = validate_liquidation_distribution_level(&request("BTC", 100.0), &level);

        assert!(result.is_err());
    }
}

use super::{ChartInstance, FundingFetchMode, FundingFetchRequest};
use crate::api::Candle;

// ---------------------------------------------------------------------------
// Funding Request Planning
// ---------------------------------------------------------------------------

const FUNDING_HISTORY_INCREMENT_MS: u64 = 60 * 60 * 1_000;
const FUNDING_INCREMENTAL_RETRY_MS: u64 = 5 * 60 * 1_000;
const FUNDING_EMPTY_SNAPSHOT_RETRY_MS: u64 = 15 * 60 * 1_000;

pub(super) enum FundingRequestPlan {
    Fetch(FundingFetchRequest),
    Wait,
    Status(String, bool),
}

pub(super) fn plan_funding_request(
    instance: &ChartInstance,
    muted: bool,
    coin: Option<String>,
    now_ms: u64,
    hydromancer_key_generation: u64,
    api_key_missing: bool,
) -> FundingRequestPlan {
    if muted {
        return FundingRequestPlan::Status("Ticker is hidden in Settings > Risk".to_string(), true);
    }
    if api_key_missing {
        return FundingRequestPlan::Status(
            "Add Hydromancer key in Settings > Integrations".to_string(),
            true,
        );
    }
    let Some(coin) = coin else {
        return FundingRequestPlan::Status("Funding requires a perp symbol".to_string(), true);
    };
    let Some((start_ms, end_ms)) = funding_time_range(
        &instance.chart.candles,
        instance.interval.duration_ms(),
        now_ms,
    ) else {
        return FundingRequestPlan::Wait;
    };

    if let Some(latest_time_ms) = instance
        .chart
        .funding_rates
        .iter()
        .map(|point| point.time_ms)
        .max()
    {
        if !funding_incremental_due(latest_time_ms, end_ms)
            || !funding_attempt_allowed(
                instance.funding_last_attempt_ms,
                now_ms,
                FUNDING_INCREMENTAL_RETRY_MS,
            )
        {
            return FundingRequestPlan::Wait;
        }

        return FundingRequestPlan::Fetch(FundingFetchRequest {
            chart_id: instance.id,
            symbol: instance.symbol.clone(),
            coin,
            hydromancer_key_generation,
            start_ms: latest_time_ms,
            end_ms,
            mode: FundingFetchMode::Incremental,
        });
    }

    if !funding_attempt_allowed(
        instance.funding_last_attempt_ms,
        now_ms,
        FUNDING_EMPTY_SNAPSHOT_RETRY_MS,
    ) {
        return FundingRequestPlan::Wait;
    }

    FundingRequestPlan::Fetch(FundingFetchRequest {
        chart_id: instance.id,
        symbol: instance.symbol.clone(),
        coin,
        hydromancer_key_generation,
        start_ms,
        end_ms,
        mode: FundingFetchMode::Snapshot,
    })
}

pub(super) fn funding_time_range(
    candles: &[Candle],
    interval_ms: u64,
    now_ms: u64,
) -> Option<(u64, u64)> {
    let first = candles.first()?.open_time;
    let last = candles.last()?.open_time;
    let end = last.saturating_add(interval_ms).min(now_ms.max(last));
    (end > first).then_some((first, end))
}

pub(super) fn funding_incremental_due(latest_time_ms: u64, target_end_ms: u64) -> bool {
    target_end_ms >= latest_time_ms.saturating_add(FUNDING_HISTORY_INCREMENT_MS)
}

pub(super) fn funding_attempt_allowed(
    last_attempt_ms: Option<u64>,
    now_ms: u64,
    min_interval_ms: u64,
) -> bool {
    match last_attempt_ms {
        Some(last) => now_ms.saturating_sub(last) >= min_interval_ms,
        None => true,
    }
}

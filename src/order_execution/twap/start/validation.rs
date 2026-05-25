use crate::twap_state::{
    parse_twap_duration_minutes, parse_twap_slice_count, twap_aggregate_schedule_has_capacity,
    twap_aggregate_slice_rate, validate_twap_interval,
};

use std::time::Duration;

// ---------------------------------------------------------------------------
// TWAP Start Validation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(super) struct TwapStartSchedule {
    pub(super) duration: Duration,
    pub(super) slice_count: u32,
}

pub(super) fn parse_twap_start_schedule(
    duration_minutes: &str,
    slices: &str,
) -> Result<TwapStartSchedule, String> {
    let Some(duration) = parse_twap_duration_minutes(duration_minutes) else {
        return Err("Invalid TWAP duration: use 1 minute to 24 hours".to_string());
    };
    let Some(slice_count) = parse_twap_slice_count(slices) else {
        return Err(format!(
            "Invalid TWAP slices: use 1 to {}",
            crate::twap_state::TWAP_MAX_SLICES
        ));
    };
    if !validate_twap_interval(duration, slice_count) {
        return Err("TWAP interval is too short: use at least 5 seconds per slice".to_string());
    }

    Ok(TwapStartSchedule {
        duration,
        slice_count,
    })
}

pub(super) fn validate_twap_schedule_capacity(
    active_slice_rate: f64,
    duration: Duration,
    slice_count: u32,
) -> Result<(), String> {
    if twap_aggregate_schedule_has_capacity(active_slice_rate, duration, slice_count) {
        return Ok(());
    }

    let combined_rate =
        twap_aggregate_slice_rate(active_slice_rate, duration, slice_count).unwrap_or(0.0);
    Err(format!(
        concat!(
            "Cannot start TWAP: active TWAP schedule is too dense ",
            "({:.2} slices/sec). Increase duration, reduce slices, ",
            "or wait for another TWAP to finish."
        ),
        combined_rate
    ))
}

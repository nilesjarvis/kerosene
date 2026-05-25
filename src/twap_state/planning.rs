use super::{
    MIN_EXCHANGE_ORDER_NOTIONAL_USD, TWAP_MAX_AGGREGATE_SLICE_RATE, TWAP_MAX_DURATION,
    TWAP_MAX_SLICES, TWAP_MIN_DURATION, TWAP_MIN_INTERVAL,
};
use crate::api::OrderBook;
use crate::helpers::positive_finite_value;
use std::time::Duration;

// ---------------------------------------------------------------------------
// TWAP Planning Helpers
// ---------------------------------------------------------------------------

pub(crate) fn parse_twap_duration_minutes(value: &str) -> Option<Duration> {
    let minutes = value.trim().parse::<f64>().ok()?;
    let minutes = positive_finite_value(minutes)?;
    let duration = Duration::from_secs_f64(minutes * 60.0);
    (duration >= TWAP_MIN_DURATION && duration <= TWAP_MAX_DURATION).then_some(duration)
}

pub(crate) fn parse_twap_slice_count(value: &str) -> Option<u32> {
    let count = value.trim().parse::<u32>().ok()?;
    (count > 0 && count <= TWAP_MAX_SLICES).then_some(count)
}

pub(crate) fn validate_twap_interval(duration: Duration, slice_count: u32) -> bool {
    slice_count > 0 && duration / slice_count >= TWAP_MIN_INTERVAL
}

pub(crate) fn twap_target_size_from_quantity(
    raw_quantity: f64,
    reference_price: Option<f64>,
    quantity_is_usd: bool,
) -> Option<f64> {
    let raw_quantity = positive_finite_value(raw_quantity)?;
    let target_size = if quantity_is_usd {
        let reference_price = positive_finite_value(reference_price?)?;
        raw_quantity / reference_price
    } else {
        raw_quantity
    };
    positive_finite_value(target_size)
}

pub(crate) fn twap_required_slice_rate(duration: Duration, slice_count: u32) -> Option<f64> {
    if slice_count == 0 {
        return None;
    }
    let seconds = duration.as_secs_f64();
    if !seconds.is_finite() || seconds <= 0.0 {
        return Some(f64::INFINITY);
    }
    Some(f64::from(slice_count) / seconds)
}

pub(crate) fn twap_aggregate_slice_rate(
    active_slice_rate: f64,
    duration: Duration,
    slice_count: u32,
) -> Option<f64> {
    if !active_slice_rate.is_finite() || active_slice_rate < 0.0 {
        return None;
    }
    let new_rate = twap_required_slice_rate(duration, slice_count)?;
    let total_rate = active_slice_rate + new_rate;
    (total_rate.is_finite() && total_rate >= 0.0).then_some(total_rate)
}

pub(crate) fn twap_aggregate_schedule_has_capacity(
    active_slice_rate: f64,
    duration: Duration,
    slice_count: u32,
) -> bool {
    twap_aggregate_slice_rate(active_slice_rate, duration, slice_count)
        .is_some_and(|rate| rate <= TWAP_MAX_AGGREGATE_SLICE_RATE + f64::EPSILON)
}

pub(crate) fn twap_min_quantized_child_notional(
    target_size: f64,
    slice_count: u32,
    min_price: f64,
    randomize: bool,
    sz_decimals: u32,
) -> Option<f64> {
    if slice_count == 0 {
        return None;
    }
    let target_size = positive_finite_value(target_size)?;
    let min_price = positive_finite_value(min_price)?;
    let base_size = target_size / f64::from(slice_count);
    let min_size = if randomize {
        base_size * 0.8
    } else {
        base_size
    };
    let quantized_size = quantize_twap_slice_size(min_size, target_size, sz_decimals)?;
    let notional = quantized_size * min_price;
    positive_finite_value(notional)
}

pub(crate) fn twap_order_notional_meets_minimum(size: f64, price: f64) -> bool {
    positive_finite_value(size)
        .zip(positive_finite_value(price))
        .is_some_and(|(size, price)| size * price >= MIN_EXCHANGE_ORDER_NOTIONAL_USD)
}

pub(crate) fn quantize_twap_slice_size(
    size: f64,
    remaining_size: f64,
    sz_decimals: u32,
) -> Option<f64> {
    let size = positive_finite_value(size)?;
    let remaining_size = positive_finite_value(remaining_size)?;
    let decimals = sz_decimals.min(8);
    let factor = 10f64.powi(decimals as i32);
    let max_size = (remaining_size * factor).floor() / factor;
    let quantized = ((size.min(remaining_size)) * factor).floor() / factor;
    let quantized = quantized.min(max_size);
    positive_finite_value(quantized)
}

pub(crate) fn twap_limit_price_for_slice(
    book: &OrderBook,
    is_buy: bool,
    planned_size: f64,
    min_price: f64,
    max_price: f64,
) -> Option<f64> {
    let planned_size = positive_finite_value(planned_size)?;
    let min_price = positive_finite_value(min_price)?;
    if !max_price.is_finite() || max_price <= min_price {
        return None;
    }

    let levels = if is_buy { &book.asks } else { &book.bids };
    let best = levels.first()?.px;
    if !best.is_finite() || best < min_price || best > max_price {
        return None;
    }

    let mut cumulative_size = 0.0;
    for level in levels {
        let price = level.px;
        let Some(size) = positive_finite_value(level.sz) else {
            continue;
        };
        let Some(price) = positive_finite_value(price) else {
            continue;
        };
        if price < min_price || price > max_price {
            break;
        }
        cumulative_size += size;
        if cumulative_size >= planned_size {
            return Some(price);
        }
    }
    None
}

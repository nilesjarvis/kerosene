use super::{
    MIN_EXCHANGE_ORDER_NOTIONAL_USD, TWAP_MAX_AGGREGATE_SLICE_RATE, next_slice,
    parse_twap_duration_minutes, parse_twap_slice_count, positive_child_notional,
    quantize_twap_slice_size, test_twap_order, twap_aggregate_schedule_has_capacity,
    twap_aggregate_slice_rate, twap_min_quantized_child_notional,
    twap_order_notional_meets_minimum, twap_required_slice_rate, twap_target_size_from_quantity,
    valid_duration_minutes, validate_twap_interval,
};

use std::time::{Duration, Instant};

mod notional;
mod schedule;
mod slices;
mod target;

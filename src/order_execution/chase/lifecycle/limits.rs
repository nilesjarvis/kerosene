use crate::helpers::positive_finite_value;
use crate::order_execution::order_account_addresses_match;
use crate::signing::{
    ChaseOrder, MAX_CHASE_DRIFT_FRACTION, MAX_CHASE_DURATION, MAX_CHASE_REPRICES,
};

use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Chase Reprice Limits
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ChaseLimitReason {
    InvalidPrice,
    Timeout { elapsed: Duration },
    MaxReprices { count: u32 },
    Drift { drift_fraction: f64 },
}

impl ChaseLimitReason {
    pub(super) fn status_detail(self) -> String {
        match self {
            Self::InvalidPrice => "invalid chase price".to_string(),
            Self::Timeout { elapsed } => {
                format!("timeout after {}s", elapsed.as_secs())
            }
            Self::MaxReprices { count } => {
                format!("max reprice count reached ({count}/{MAX_CHASE_REPRICES})")
            }
            Self::Drift { drift_fraction } => format!(
                "price drift limit exceeded ({:.2}% > {:.2}%)",
                drift_fraction * 100.0,
                MAX_CHASE_DRIFT_FRACTION * 100.0
            ),
        }
    }
}

pub(super) fn chase_account_matches(chase: &ChaseOrder, connected_address: Option<&str>) -> bool {
    connected_address
        .is_some_and(|connected| order_account_addresses_match(connected, &chase.account_address))
}

pub(super) fn chase_reprice_limit_reason(
    chase: &ChaseOrder,
    next_price: f64,
    now: Instant,
) -> Option<ChaseLimitReason> {
    if positive_finite_value(chase.initial_price).is_none()
        || positive_finite_value(next_price).is_none()
    {
        return Some(ChaseLimitReason::InvalidPrice);
    }

    let elapsed = now.saturating_duration_since(chase.started_at);
    if elapsed >= MAX_CHASE_DURATION {
        return Some(ChaseLimitReason::Timeout { elapsed });
    }

    if chase.reprice_count >= MAX_CHASE_REPRICES {
        return Some(ChaseLimitReason::MaxReprices {
            count: chase.reprice_count,
        });
    }

    let drift_fraction = (next_price - chase.initial_price).abs() / chase.initial_price;
    if drift_fraction > MAX_CHASE_DRIFT_FRACTION {
        return Some(ChaseLimitReason::Drift { drift_fraction });
    }

    None
}

mod actions;
mod client;
mod crypto;
mod model;
mod numbers;

pub use client::{
    PlaceOrderRequest, cancel_order, cancel_order_by_cloid, modify_order, place_order_with_cloid,
    update_leverage,
};
pub use model::{
    CHASE_RETRY_COOLDOWN, ChaseLifecycle, ChaseOrder, ChaseQueuedAction, ChaseStopPhase,
    ChaseVerificationReason, ExchangeOrderKind, ExchangeResponse, MAX_CHASE_CANCEL_RETRIES,
    MAX_CHASE_DRIFT_FRACTION, MAX_CHASE_DURATION, MAX_CHASE_REPRICES, MIN_CHASE_REPRICE_INTERVAL,
    OrderKind, chase_place_cloid,
};
pub use numbers::{float_to_wire, round_price};

#[cfg(test)]
mod tests;

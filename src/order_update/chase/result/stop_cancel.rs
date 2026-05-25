use crate::signing::{ChaseOrder, ExchangeResponse};

use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Stopped Chase Cancel Planning
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StoppedChaseCancelRequest {
    pub(super) chase_id: u64,
    pub(super) agent_key: Zeroizing<String>,
    pub(super) asset: u32,
    pub(super) oid: u64,
}

pub(super) fn stopped_chase_cancel_request(
    chase: &ChaseOrder,
    response: &ExchangeResponse,
) -> Option<StoppedChaseCancelRequest> {
    if !chase.lifecycle.is_stopping() || response.is_error() || response.is_fully_filled() {
        return None;
    }
    let agent_key = chase.agent_key.trim();
    if agent_key.is_empty() {
        return None;
    }
    Some(StoppedChaseCancelRequest {
        chase_id: chase.id,
        agent_key: agent_key.to_string().into(),
        asset: chase.asset,
        oid: response.order_oid()?,
    })
}

use crate::message::Message;
use crate::order_execution::{cancel_order_by_cloid_task, cancel_order_task};
use crate::twap_state::TwapChildOrder;

use iced::Task;

// ---------------------------------------------------------------------------
// TWAP Cancellation Helpers
// ---------------------------------------------------------------------------

pub(in crate::order_execution::twap) fn twap_cancel_target_matches(
    pending_oid: Option<u64>,
    pending_cloid: Option<&str>,
    oid: Option<u64>,
    cloid: Option<&str>,
) -> bool {
    oid.is_some() && pending_oid == oid
        || cloid.is_some() && pending_cloid == cloid
        || pending_oid.is_none() && oid.is_none() && pending_cloid == cloid
}

pub(in crate::order_execution::twap) fn twap_child_matches_cancel_target(
    child: &TwapChildOrder,
    oid: Option<u64>,
    cloid: Option<&str>,
) -> bool {
    oid.is_some() && child.oid == oid || cloid.is_some() && child.cloid.as_deref() == cloid
}

pub(in crate::order_execution::twap) fn twap_cancel_label(
    oid: Option<u64>,
    cloid: Option<&str>,
) -> String {
    match (oid, cloid) {
        (Some(oid), Some(cloid)) => format!("oid {oid} / {cloid}"),
        (Some(oid), None) => format!("oid {oid}"),
        (None, Some(cloid)) => cloid.to_string(),
        (None, None) => "unknown child".to_string(),
    }
}

pub(in crate::order_execution::twap) fn twap_cancel_child_task(
    twap_id: u64,
    key: String,
    asset: u32,
    oid: Option<u64>,
    cloid: Option<String>,
) -> Task<Message> {
    if key.trim().is_empty() {
        return Task::perform(
            async { Err("original agent key unavailable".to_string()) },
            move |result| Message::TwapUnexpectedCancelResult {
                twap_id,
                oid,
                cloid: cloid.clone(),
                result: Box::new(result),
            },
        );
    }

    if let Some(cloid) = cloid {
        let request_cloid = cloid.clone();
        return cancel_order_by_cloid_task(key.into(), asset, request_cloid, move |result| {
            Message::TwapUnexpectedCancelResult {
                twap_id,
                oid: None,
                cloid: Some(cloid.clone()),
                result: Box::new(result),
            }
        });
    }

    let Some(oid) = oid else {
        return Task::none();
    };
    cancel_order_task(key.into(), asset, oid, move |result| {
        Message::TwapUnexpectedCancelResult {
            twap_id,
            oid: Some(oid),
            cloid: None,
            result: Box::new(result),
        }
    })
}

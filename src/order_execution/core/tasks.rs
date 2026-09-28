use super::PreparedModifyOrder;
use crate::message::Message;
use crate::signing::{
    CapturedAgentKey, ExchangeResponse, PlaceOrderRequest, cancel_order, cancel_order_by_cloid,
    modify_order, place_order_with_cloid,
};
use iced::Task;

// ---------------------------------------------------------------------------
// Exchange Tasks
// ---------------------------------------------------------------------------

pub(crate) fn place_order_task<F>(
    key: CapturedAgentKey,
    request: PlaceOrderRequest,
    map: F,
) -> Task<Message>
where
    F: FnOnce(Result<ExchangeResponse, String>) -> Message + Send + 'static,
{
    Task::perform(place_order_with_cloid(key, request), map)
}

pub(crate) fn cancel_order_task<F>(
    key: CapturedAgentKey,
    asset: u32,
    oid: u64,
    map: F,
) -> Task<Message>
where
    F: FnOnce(Result<ExchangeResponse, String>) -> Message + Send + 'static,
{
    Task::perform(cancel_order(key, asset, oid), map)
}

pub(crate) fn cancel_order_by_cloid_task<F>(
    key: CapturedAgentKey,
    asset: u32,
    cloid: String,
    map: F,
) -> Task<Message>
where
    F: FnOnce(Result<ExchangeResponse, String>) -> Message + Send + 'static,
{
    Task::perform(cancel_order_by_cloid(key, asset, cloid), map)
}

pub(crate) fn modify_order_task<F>(
    key: CapturedAgentKey,
    order: PreparedModifyOrder,
    map: F,
) -> Task<Message>
where
    F: FnOnce(Result<ExchangeResponse, String>) -> Message + Send + 'static,
{
    Task::perform(
        modify_order(
            key,
            order.oid,
            order.asset,
            order.is_buy,
            order.price,
            order.size,
            order.reduce_only,
        ),
        map,
    )
}

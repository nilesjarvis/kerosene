use crate::account::OpenOrder;
use crate::signing::ChaseOrder;

// ---------------------------------------------------------------------------
// Open Orders
// ---------------------------------------------------------------------------

pub(super) fn preserve_open_order_reduce_only(order: &mut OpenOrder, existing: &[OpenOrder]) {
    if order.reduce_only.is_none()
        && let Some(previous) = existing.iter().find(|previous| previous.oid == order.oid)
    {
        order.reduce_only = previous.reduce_only;
    }
}

pub(super) fn apply_open_order_to_chase(
    chase: &mut ChaseOrder,
    order: &OpenOrder,
) -> Result<bool, ()> {
    let sz = order.sz.parse::<f64>().map_err(|_| ())?;
    let oversized = chase.sync_open_remaining_size(sz).ok_or(())?;
    if !chase.remaining_size.is_finite() {
        return Err(());
    }

    chase.record_oid(order.oid);
    if let Ok(px) = order.limit_px.parse::<f64>()
        && let Some((rounded_px, price_wire)) = chase.rounded_price(px)
    {
        chase.current_price = rounded_px;
        chase.current_price_wire = price_wire;
        if chase
            .desired_price
            .and_then(|price| chase.rounded_price(price))
            .is_some_and(|(_, desired_wire)| desired_wire == chase.current_price_wire)
        {
            chase.desired_price = None;
        }
    }
    Ok(oversized)
}

pub(super) fn first_open_chase_oid(chase: &ChaseOrder, open_orders: &[OpenOrder]) -> Option<u64> {
    chase
        .current_oid
        .filter(|oid| open_orders.iter().any(|order| order.oid == *oid))
        .or_else(|| {
            open_orders
                .iter()
                .find(|order| chase.tracks_oid(order.oid))
                .map(|order| order.oid)
        })
}

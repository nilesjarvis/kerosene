use super::model::OrderKind;
use serde::Serialize;

// ----- Msgpack wire types with correct field order -----
// Field order MUST match the Python SDK exactly, because msgpack preserves
// map key order and the action hash depends on the exact bytes.

/// Order wire: fields in Python SDK order: a, b, p, s, r, t
#[derive(Debug, Clone, Serialize)]
struct OrderWire {
    a: u32,
    b: bool,
    p: String,
    s: String,
    r: bool,
    t: OrderTypeWire,
}

#[derive(Debug, Clone, Serialize)]
struct OrderTypeWire {
    limit: LimitOrderWire,
}

#[derive(Debug, Clone, Serialize)]
struct LimitOrderWire {
    tif: String,
}

/// Order action: fields in Python SDK order: type, orders, grouping
#[derive(Debug, Clone, Serialize)]
pub(super) struct OrderAction {
    #[serde(rename = "type")]
    action_type: String,
    orders: Vec<OrderWire>,
    grouping: String,
}

/// Cancel wire: fields in Python SDK order: a, o
#[derive(Debug, Clone, Serialize)]
struct CancelWire {
    a: u32,
    o: u64,
}

/// Cancel action: fields in Python SDK order: type, cancels
#[derive(Debug, Clone, Serialize)]
pub(super) struct CancelAction {
    #[serde(rename = "type")]
    action_type: String,
    cancels: Vec<CancelWire>,
}

/// Modify wire: fields in Python SDK order: oid, order
#[derive(Debug, Clone, Serialize)]
struct ModifyWire {
    oid: u64,
    order: OrderWire,
}

/// Batch modify action: fields in Python SDK order: type, modifies
#[derive(Debug, Clone, Serialize)]
pub(super) struct ModifyAction {
    #[serde(rename = "type")]
    action_type: String,
    modifies: Vec<ModifyWire>,
}

fn order_tif(order_kind: OrderKind) -> &'static str {
    match order_kind {
        OrderKind::Market | OrderKind::LimitIoc => "Ioc",
        OrderKind::Limit | OrderKind::Chase => "Gtc",
    }
}

fn build_order_wire(
    asset: u32,
    is_buy: bool,
    price: String,
    size: String,
    order_kind: OrderKind,
    reduce_only: bool,
) -> OrderWire {
    OrderWire {
        a: asset,
        b: is_buy,
        p: price,
        s: size,
        r: reduce_only,
        t: OrderTypeWire {
            limit: LimitOrderWire {
                tif: order_tif(order_kind).to_string(),
            },
        },
    }
}

pub(super) fn build_order_action(
    asset: u32,
    is_buy: bool,
    price: String,
    size: String,
    order_kind: OrderKind,
    reduce_only: bool,
) -> OrderAction {
    OrderAction {
        action_type: "order".to_string(),
        orders: vec![build_order_wire(
            asset,
            is_buy,
            price,
            size,
            order_kind,
            reduce_only,
        )],
        grouping: "na".to_string(),
    }
}

pub(super) fn build_cancel_action(asset: u32, oid: u64) -> CancelAction {
    CancelAction {
        action_type: "cancel".to_string(),
        cancels: vec![CancelWire { a: asset, o: oid }],
    }
}

pub(super) fn build_modify_action(
    oid: u64,
    asset: u32,
    is_buy: bool,
    price: String,
    size: String,
    reduce_only: bool,
) -> ModifyAction {
    ModifyAction {
        action_type: "batchModify".to_string(),
        modifies: vec![ModifyWire {
            oid,
            order: build_order_wire(asset, is_buy, price, size, OrderKind::Limit, reduce_only),
        }],
    }
}

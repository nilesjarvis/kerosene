use super::model::OrderKind;
use serde::Serialize;

// ---------------------------------------------------------------------------
// Signed Action Enum
//
// All L1 actions Kerosene signs share the same wire pipeline: msgpack → keccak
// → EIP-712 (Agent phantom type, chain 1337) → r/s/v posted to /exchange.
// The variants here are the action shapes; the dispatcher in `client.rs`
// takes any `HyperliquidL1Action` and runs the shared signing pipeline. Adding
// a new L1 action type means: one new variant (or a constructor on an existing
// variant) here, one thin wrapper in `client.rs`. No new boilerplate copy.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub(super) enum HyperliquidL1Action {
    Order(OrderAction),
    Cancel(CancelAction),
    CancelByCloid(CancelByCloidAction),
    Modify(ModifyAction),
}

impl HyperliquidL1Action {
    pub(super) fn order(
        asset: u32,
        is_buy: bool,
        price: String,
        size: String,
        order_kind: OrderKind,
        reduce_only: bool,
    ) -> Self {
        Self::Order(build_order_action(
            asset,
            is_buy,
            price,
            size,
            order_kind,
            reduce_only,
        ))
    }

    pub(super) fn order_with_cloid(
        asset: u32,
        is_buy: bool,
        price: String,
        size: String,
        order_kind: OrderKind,
        reduce_only: bool,
        cloid: Option<String>,
    ) -> Self {
        Self::Order(build_order_action_with_cloid(
            asset,
            is_buy,
            price,
            size,
            order_kind,
            reduce_only,
            cloid,
        ))
    }

    pub(super) fn cancel(asset: u32, oid: u64) -> Self {
        Self::Cancel(build_cancel_action(asset, oid))
    }

    pub(super) fn cancel_by_cloid(asset: u32, cloid: String) -> Self {
        Self::CancelByCloid(build_cancel_by_cloid_action(asset, cloid))
    }

    pub(super) fn modify(
        oid: u64,
        asset: u32,
        is_buy: bool,
        price: String,
        size: String,
        reduce_only: bool,
    ) -> Self {
        Self::Modify(build_modify_action(
            oid,
            asset,
            is_buy,
            price,
            size,
            reduce_only,
        ))
    }
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    c: Option<String>,
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

/// Cancel-by-cloid wire: fields in docs order: asset, cloid
#[derive(Debug, Clone, Serialize)]
struct CancelByCloidWire {
    asset: u32,
    cloid: String,
}

/// Cancel-by-cloid action: fields in docs order: type, cancels
#[derive(Debug, Clone, Serialize)]
pub(super) struct CancelByCloidAction {
    #[serde(rename = "type")]
    action_type: String,
    cancels: Vec<CancelByCloidWire>,
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
    cloid: Option<String>,
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
        c: cloid,
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
    build_order_action_with_cloid(asset, is_buy, price, size, order_kind, reduce_only, None)
}

pub(super) fn build_order_action_with_cloid(
    asset: u32,
    is_buy: bool,
    price: String,
    size: String,
    order_kind: OrderKind,
    reduce_only: bool,
    cloid: Option<String>,
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
            cloid,
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

pub(super) fn build_cancel_by_cloid_action(asset: u32, cloid: String) -> CancelByCloidAction {
    CancelByCloidAction {
        action_type: "cancelByCloid".to_string(),
        cancels: vec![CancelByCloidWire { asset, cloid }],
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
            order: build_order_wire(
                asset,
                is_buy,
                price,
                size,
                OrderKind::Limit,
                reduce_only,
                None,
            ),
        }],
    }
}

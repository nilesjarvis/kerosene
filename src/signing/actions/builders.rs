use super::wire::{
    CancelAction, CancelByCloidAction, CancelByCloidWire, CancelWire, LimitOrderWire, ModifyAction,
    ModifyWire, OrderAction, OrderTypeWire, OrderWire, UpdateLeverageAction,
};
use crate::signing::model::OrderKind;

// ---------------------------------------------------------------------------
// Action Builders
// ---------------------------------------------------------------------------

fn order_tif(order_kind: OrderKind) -> &'static str {
    match order_kind {
        OrderKind::Market | OrderKind::Twap | OrderKind::LimitIoc => "Ioc",
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

pub(in crate::signing) fn build_order_action(
    asset: u32,
    is_buy: bool,
    price: String,
    size: String,
    order_kind: OrderKind,
    reduce_only: bool,
) -> OrderAction {
    build_order_action_with_cloid(asset, is_buy, price, size, order_kind, reduce_only, None)
}

pub(in crate::signing) fn build_order_action_with_cloid(
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

pub(in crate::signing) fn build_cancel_action(asset: u32, oid: u64) -> CancelAction {
    CancelAction {
        action_type: "cancel".to_string(),
        cancels: vec![CancelWire { a: asset, o: oid }],
    }
}

pub(in crate::signing) fn build_cancel_by_cloid_action(
    asset: u32,
    cloid: String,
) -> CancelByCloidAction {
    CancelByCloidAction {
        action_type: "cancelByCloid".to_string(),
        cancels: vec![CancelByCloidWire { asset, cloid }],
    }
}

pub(in crate::signing) fn build_modify_action(
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

pub(in crate::signing) fn build_update_leverage_action(
    asset: u32,
    is_cross: bool,
    leverage: u32,
) -> UpdateLeverageAction {
    UpdateLeverageAction {
        action_type: "updateLeverage".to_string(),
        asset,
        is_cross,
        leverage,
    }
}

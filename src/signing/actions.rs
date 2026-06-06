use super::model::ExchangeOrderKind;
use wire::{CancelAction, CancelByCloidAction, ModifyAction, OrderAction, UpdateLeverageAction};

mod builders;
mod wire;

#[cfg(test)]
pub(super) use builders::build_order_action;
pub(super) use builders::{
    build_cancel_action, build_cancel_by_cloid_action, build_modify_action,
    build_order_action_with_cloid, build_update_leverage_action,
};
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
    UpdateLeverage(UpdateLeverageAction),
}

impl HyperliquidL1Action {
    #[cfg(test)]
    pub(super) fn order(
        asset: u32,
        is_buy: bool,
        price: String,
        size: String,
        order_kind: ExchangeOrderKind,
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
        order_kind: ExchangeOrderKind,
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

    pub(super) fn update_leverage(asset: u32, is_cross: bool, leverage: u32) -> Self {
        Self::UpdateLeverage(build_update_leverage_action(asset, is_cross, leverage))
    }
}

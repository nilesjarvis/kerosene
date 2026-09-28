mod account_context;
mod active_symbol;
mod advanced;
mod chase;
mod core;
mod exchange_errors;
mod hud;
mod identities;
mod pending;
mod position_actions;
pub(crate) mod pricing;
mod quick_order;
mod quick_trade;
mod resting_order;
mod sizing;
mod submit;
mod symbols;
mod twap;

pub(crate) use account_context::order_account_addresses_match;
pub(crate) use advanced::{AdvancedOrderKind, AdvancedOrderStartSnapshot, TwapOrderStartSnapshot};
pub(crate) use core::{
    CancelIntent, MarketUsdSizeReference, ModifyIntent, OneShotPlacementContext, OrderOperation,
    OrderSurface, PlaceIntent, PreparedExchangeOrder, PreparedModifyOrder,
    PreparedModifyOrderResult, PriceSource, QuantityDenomination, QuantitySource, ReduceOnlySource,
    cancel_order_by_cloid_task, cancel_order_task, modify_order_task, place_order_task,
    validate_surface_market_type,
};
pub(crate) use exchange_errors::{cancel_error_indicates_closed_order, retryable_exchange_error};
pub(crate) use hud::{
    HudOrderRequest, HudOrderSide, HudOrderType, HudPlacementTracker, MAX_INFLIGHT_HUD_PLACEMENTS,
};
pub(crate) use identities::{
    SpotAutomationSymbolIdentity, open_order_matches_chase_identity, open_order_side_is_buy,
};
pub(crate) use pending::{
    OrderLeverageSubmissionSnapshot, PendingLeverageUpdateContext, PendingNukeExecution,
    PendingOrderAction,
};
pub(crate) use position_actions::{NukePlan, reject_if_positions_incomplete_for_action};
pub(crate) use quick_order::{
    MoveOrderKey, PendingMoveOrderContext, QuickOrderForm, QuickOrderQuantityProvenance,
    QuickOrderRecovery, QuickOrderSubmissionSnapshot,
};
pub(crate) use quick_trade::QuickTradeOrderRequest;
pub(crate) use resting_order::{RestingOrderWireError, validate_resting_order_wire};
pub(crate) use sizing::order_size_from_quantity_input;
pub(crate) use submit::{TicketOrderPlaceIntent, TicketOrderSubmissionSnapshot};

#[cfg(test)]
pub(crate) use position_actions::{NukePositionOrder, NukeSkipReason};
#[cfg(test)]
pub(crate) use quick_order::MoveOrderContextError;

use crate::app_state::TradingTerminal;
use crate::order_execution::{OneShotPlacementContext, OrderSurface};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlacementLookup {
    Checking,
    Unconfirmed,
    Canceled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingOneShotStatusRequest {
    pub(crate) request_id: u64,
    pub(crate) context: OneShotPlacementContext,
    pub(crate) lookup: PlacementLookup,
    // A snapshot releases the existing account-wide gate. The standard ticket
    // also needs a result for its own client order ID before another placement.
    account_refreshed: bool,
}

impl PendingOneShotStatusRequest {
    pub(crate) fn new(request_id: u64, context: &OneShotPlacementContext) -> Self {
        Self {
            request_id,
            context: context.clone(),
            lookup: PlacementLookup::Checking,
            account_refreshed: false,
        }
    }

    pub(super) fn matches(&self, request_id: u64, context: &OneShotPlacementContext) -> bool {
        self.request_id == request_id && &self.context == context
    }

    pub(super) fn is_for_context(&self, context: &OneShotPlacementContext) -> bool {
        self.context.account_address == context.account_address
            && self.context.cloid == context.cloid
    }

    pub(crate) fn is_for_account(&self, account_address: &str) -> bool {
        crate::order_execution::order_account_addresses_match(
            &self.context.account_address,
            account_address,
        )
    }

    pub(crate) fn surface(&self) -> OrderSurface {
        self.context.surface
    }

    pub(crate) fn blocks_trading_request(&self) -> bool {
        !self.account_refreshed
    }

    pub(crate) fn needs_ticket_confirmation(&self) -> bool {
        self.context.surface.is_standard_ticket() && self.lookup != PlacementLookup::Canceled
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PendingCancelStatusRequest {
    account_address: String,
    oid: u64,
    symbol: String,
}

impl fmt::Debug for PendingCancelStatusRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PendingCancelStatusRequest")
            .field("account_address", &"<redacted>")
            .field("oid", &"<redacted>")
            .field("symbol", &self.symbol)
            .finish()
    }
}

impl PendingCancelStatusRequest {
    pub(crate) fn new(account_address: String, oid: u64, symbol: String) -> Self {
        Self {
            account_address,
            oid,
            symbol,
        }
    }

    pub(crate) fn oid(&self) -> u64 {
        self.oid
    }

    pub(crate) fn symbol(&self) -> &str {
        &self.symbol
    }

    pub(super) fn matches(&self, account_address: &str, oid: u64, symbol: &str) -> bool {
        crate::order_execution::order_account_addresses_match(
            &self.account_address,
            account_address,
        ) && self.oid == oid
            && self.symbol == symbol
    }

    pub(super) fn is_for_account(&self, account_address: &str) -> bool {
        crate::order_execution::order_account_addresses_match(
            &self.account_address,
            account_address,
        )
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PendingMoveStatusRequest {
    account_address: String,
    oid: u64,
    symbol: String,
}

impl fmt::Debug for PendingMoveStatusRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PendingMoveStatusRequest")
            .field("account_address", &"<redacted>")
            .field("oid", &"<redacted>")
            .field("symbol", &self.symbol)
            .finish()
    }
}

impl PendingMoveStatusRequest {
    pub(crate) fn new(account_address: String, oid: u64, symbol: String) -> Self {
        Self {
            account_address,
            oid,
            symbol,
        }
    }

    pub(crate) fn matches(&self, account_address: &str, oid: u64, symbol: &str) -> bool {
        crate::order_execution::order_account_addresses_match(
            &self.account_address,
            account_address,
        ) && self.oid == oid
            && self.symbol == symbol
    }

    fn is_for_account(&self, account_address: &str) -> bool {
        crate::order_execution::order_account_addresses_match(
            &self.account_address,
            account_address,
        )
    }
}

impl TradingTerminal {
    pub(crate) fn insert_pending_one_shot_status_request(
        &mut self,
        request: PendingOneShotStatusRequest,
    ) {
        self.pending_one_shot_status_requests
            .insert(request.request_id, request);
    }

    #[cfg(test)]
    pub(crate) fn has_pending_one_shot_status_requests_for_test(&self) -> bool {
        !self.pending_one_shot_status_requests.is_empty()
    }

    pub(crate) fn reconcile_placement_checks_after_refresh(&mut self, account_address: &str) {
        self.pending_one_shot_status_requests.retain(|_, pending| {
            if !pending.is_for_account(account_address) {
                return true;
            }
            pending.account_refreshed = true;
            pending.needs_ticket_confirmation()
        });
    }

    pub(crate) fn retain_unconfirmed_ticket_checks(&mut self) {
        self.pending_one_shot_status_requests.retain(|_, pending| {
            pending.needs_ticket_confirmation() && !pending.blocks_trading_request()
        });
    }

    pub(crate) fn clear_pending_order_status_requests_for_account_after_refresh(
        &mut self,
        account_address: &str,
    ) {
        let open_orders_complete = self
            .account_data_for_order_account(account_address)
            .is_some_and(|data| data.completeness.open_orders_complete);
        if !open_orders_complete {
            return;
        }

        if self
            .pending_cancel_status_request
            .as_ref()
            .is_some_and(|pending| pending.is_for_account(account_address))
        {
            self.pending_cancel_status_request = None;
        }
        if self
            .pending_move_status_request
            .as_ref()
            .is_some_and(|pending| pending.is_for_account(account_address))
        {
            self.pending_move_status_request = None;
        }
    }
}

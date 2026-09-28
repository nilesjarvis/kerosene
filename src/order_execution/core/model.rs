use super::cloid::next_one_shot_place_cloid;
use crate::api::MarketType;
use crate::signing::{ExchangeOrderKind, PlaceOrderRequest};
use std::fmt;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Execution Intents and Prepared Requests
// ---------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum OrderSurface {
    Ticket,
    Preset,
    QuickOrder,
    QuickTrade,
    Hud,
    ClosePosition,
    Cluster,
    ClusterClose,
    Nuke,
    Chase,
    Twap,
    Move,
    Cancel,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum OrderOperation {
    Place,
    Cancel,
    Modify,
    UpdateLeverage,
}

#[derive(Clone, PartialEq)]
pub(crate) struct PlaceIntent {
    pub(crate) surface: OrderSurface,
    pub(crate) symbol_key: String,
    pub(crate) is_buy: bool,
    pub(crate) order_kind: ExchangeOrderKind,
    pub(crate) price_source: PriceSource,
    pub(crate) quantity_source: QuantitySource,
    pub(crate) reduce_only_source: ReduceOnlySource,
}

impl fmt::Debug for PlaceIntent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlaceIntent")
            .field("surface", &self.surface)
            .field("symbol_key", &format_args!("<redacted>"))
            .field("is_buy", &self.is_buy)
            .field("order_kind", &self.order_kind)
            .field("price_source", &self.price_source)
            .field("quantity_source", &self.quantity_source)
            .field("reduce_only_source", &self.reduce_only_source)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CancelIntent {
    pub(crate) surface: OrderSurface,
    pub(crate) symbol_key: String,
    pub(crate) oid: u64,
}

impl fmt::Debug for CancelIntent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CancelIntent")
            .field("surface", &self.surface)
            .field("symbol_key", &format_args!("<redacted>"))
            .field("oid", &format_args!("<redacted>"))
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct ModifyIntent {
    pub(crate) surface: OrderSurface,
    pub(crate) symbol_key: String,
    pub(crate) oid: u64,
    pub(crate) is_buy: bool,
    pub(crate) new_price: f64,
    pub(crate) original_price: String,
    pub(crate) size: String,
    pub(crate) invalid_size_message: &'static str,
    pub(crate) reduce_only: Option<bool>,
    pub(crate) reduce_only_missing_message: &'static str,
    pub(crate) invalid_price_message: &'static str,
}

impl fmt::Debug for ModifyIntent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModifyIntent")
            .field("surface", &self.surface)
            .field("symbol_key", &format_args!("<redacted>"))
            .field("oid", &format_args!("<redacted>"))
            .field("is_buy", &self.is_buy)
            .field("new_price", &format_args!("<redacted>"))
            .field("original_price", &format_args!("<redacted>"))
            .field("size", &format_args!("<redacted>"))
            .field("reduce_only", &self.reduce_only)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum PriceSource {
    LimitInput {
        value: String,
        invalid_message: &'static str,
    },
    MarketWithSlippage {
        invalid_message: Option<&'static str>,
        usd_size_reference: MarketUsdSizeReference,
    },
    ReferenceMid,
}

impl fmt::Debug for PriceSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LimitInput {
                invalid_message, ..
            } => f
                .debug_struct("LimitInput")
                .field("value", &format_args!("<redacted>"))
                .field("invalid_message", invalid_message)
                .finish(),
            Self::MarketWithSlippage {
                invalid_message,
                usd_size_reference,
            } => f
                .debug_struct("MarketWithSlippage")
                .field("invalid_message", invalid_message)
                .field("usd_size_reference", usd_size_reference)
                .finish(),
            Self::ReferenceMid => f.write_str("ReferenceMid"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarketUsdSizeReference {
    ExecutionPrice,
    Mid,
}

#[derive(Clone, PartialEq)]
pub(crate) enum QuantitySource {
    UserInput {
        value: String,
        denomination: QuantityDenomination,
        invalid_message: &'static str,
        precision_invalid_message: &'static str,
    },
    CoinSize {
        size: f64,
        invalid_message: &'static str,
        precision_invalid_message: &'static str,
    },
    /// Exact spot percentage sizing. `available_balance` is quote-token
    /// spendable balance for buys and sellable base-token balance for sells.
    /// The final coin size is derived from the actual submitted price, never
    /// from the rounded display quantity.
    SpotPercentageBalance {
        available_balance: f64,
        percentage: f32,
        invalid_message: &'static str,
        precision_invalid_message: &'static str,
    },
}

impl fmt::Debug for QuantitySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UserInput {
                denomination,
                invalid_message,
                precision_invalid_message,
                ..
            } => f
                .debug_struct("UserInput")
                .field("value", &format_args!("<redacted>"))
                .field("denomination", denomination)
                .field("invalid_message", invalid_message)
                .field("precision_invalid_message", precision_invalid_message)
                .finish(),
            Self::CoinSize {
                invalid_message,
                precision_invalid_message,
                ..
            } => f
                .debug_struct("CoinSize")
                .field("size", &format_args!("<redacted>"))
                .field("invalid_message", invalid_message)
                .field("precision_invalid_message", precision_invalid_message)
                .finish(),
            Self::SpotPercentageBalance {
                invalid_message,
                precision_invalid_message,
                ..
            } => f
                .debug_struct("SpotPercentageBalance")
                .field("available_balance", &format_args!("<redacted>"))
                .field("percentage", &format_args!("<redacted>"))
                .field("invalid_message", invalid_message)
                .field("precision_invalid_message", precision_invalid_message)
                .finish(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuantityDenomination {
    Coin,
    UsdNotional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReduceOnlySource {
    Form(bool),
    Fixed(bool),
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OneShotPlacementContext {
    pub(crate) account_address: String,
    pub(crate) cloid: String,
    pub(crate) surface: OrderSurface,
    pub(crate) symbol_key: String,
    pub(crate) order_kind: ExchangeOrderKind,
}

impl fmt::Debug for OneShotPlacementContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OneShotPlacementContext")
            .field("account_address", &"<redacted>")
            .field("cloid", &format_args!("<redacted>"))
            .field("surface", &self.surface)
            .field("symbol_key", &format_args!("<redacted>"))
            .field("order_kind", &self.order_kind)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PreparedCancelOrder {
    pub(crate) surface: OrderSurface,
    pub(crate) symbol_key: String,
    pub(crate) asset: u32,
    pub(crate) oid: u64,
    pub(crate) market_type: MarketType,
}

impl fmt::Debug for PreparedCancelOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedCancelOrder")
            .field("surface", &self.surface)
            .field("symbol_key", &format_args!("<redacted>"))
            .field("asset", &self.asset)
            .field("oid", &format_args!("<redacted>"))
            .field("market_type", &self.market_type)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PreparedModifyOrder {
    pub(crate) surface: OrderSurface,
    pub(crate) symbol_key: String,
    pub(crate) oid: u64,
    pub(crate) asset: u32,
    pub(crate) is_buy: bool,
    pub(crate) price: String,
    pub(crate) size: String,
    pub(crate) reduce_only: bool,
    pub(crate) market_type: MarketType,
}

impl fmt::Debug for PreparedModifyOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedModifyOrder")
            .field("surface", &self.surface)
            .field("symbol_key", &format_args!("<redacted>"))
            .field("oid", &format_args!("<redacted>"))
            .field("asset", &self.asset)
            .field("is_buy", &self.is_buy)
            .field("price", &format_args!("<redacted>"))
            .field("size", &format_args!("<redacted>"))
            .field("reduce_only", &self.reduce_only)
            .field("market_type", &self.market_type)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreparedModifyOrderResult {
    Prepared(PreparedModifyOrder),
    NoPriceChange,
}

impl OneShotPlacementContext {
    pub(crate) fn placement_label(&self) -> &'static str {
        self.surface.label()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PreparedExchangeOrder {
    pub(crate) surface: OrderSurface,
    pub(crate) symbol_key: String,
    pub(crate) asset: u32,
    pub(crate) is_buy: bool,
    pub(crate) price: String,
    pub(crate) size: String,
    pub(crate) order_kind: ExchangeOrderKind,
    pub(crate) reduce_only: bool,
    pub(crate) market_type: MarketType,
}

impl fmt::Debug for PreparedExchangeOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedExchangeOrder")
            .field("surface", &self.surface)
            .field("symbol_key", &format_args!("<redacted>"))
            .field("asset", &self.asset)
            .field("is_buy", &self.is_buy)
            .field("price", &format_args!("<redacted>"))
            .field("size", &format_args!("<redacted>"))
            .field("order_kind", &self.order_kind)
            .field("reduce_only", &self.reduce_only)
            .field("market_type", &self.market_type)
            .finish()
    }
}

impl PreparedExchangeOrder {
    pub(crate) fn place_request_with_existing_cloid(&self, cloid: String) -> PlaceOrderRequest {
        PlaceOrderRequest {
            asset: self.asset,
            is_buy: self.is_buy,
            price: self.price.clone(),
            size: self.size.clone(),
            order_kind: self.order_kind,
            reduce_only: self.reduce_only,
            cloid: Some(cloid),
        }
    }

    pub(crate) fn place_request_with_context(
        &self,
        account_address: &str,
    ) -> (PlaceOrderRequest, OneShotPlacementContext) {
        let cloid = next_one_shot_place_cloid(account_address, self);
        let request = self.place_request_with_existing_cloid(cloid.clone());
        let context = OneShotPlacementContext {
            account_address: account_address.to_string(),
            cloid,
            surface: self.surface,
            symbol_key: self.symbol_key.clone(),
            order_kind: self.order_kind,
        };
        (request, context)
    }
}

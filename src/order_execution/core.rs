use super::pricing::{rounded_market_price, slipped_market_price};
use super::sizing::order_size_from_quantity_input;
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::app_time::now_ms;
use crate::helpers::{finite_value, parse_positive_number, positive_finite_value};
use crate::message::Message;
use crate::signing::{
    ExchangeOrderKind, ExchangeResponse, PlaceOrderRequest, cancel_order, cancel_order_by_cloid,
    float_to_wire, modify_order, place_order_with_cloid, round_price,
};
use iced::Task;
use sha3::{Digest, Keccak256};
use std::fmt::{self, Write as _};
use std::sync::atomic::{AtomicU64, Ordering};
use zeroize::Zeroizing;

static LAST_ONE_SHOT_CLOID_NONCE_MS: AtomicU64 = AtomicU64::new(0);

// ---------------------------------------------------------------------------
// Shared Execution Core
// ---------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum OrderSurface {
    Ticket,
    Preset,
    QuickOrder,
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
        let request = PlaceOrderRequest {
            asset: self.asset,
            is_buy: self.is_buy,
            price: self.price.clone(),
            size: self.size.clone(),
            order_kind: self.order_kind,
            reduce_only: self.reduce_only,
            cloid: Some(cloid.clone()),
        };
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrderCapabilityError {
    UnsupportedMarketType {
        surface: OrderSurface,
        operation: OrderOperation,
        market_type: MarketType,
    },
}

impl OrderCapabilityError {
    pub(crate) fn status_text(self) -> String {
        match self {
            Self::UnsupportedMarketType {
                surface,
                operation,
                market_type,
            } => match market_type {
                MarketType::Outcome => format!(
                    "Outcome {} is not available from this control; use the main order ticket",
                    surface.outcome_action_label(operation)
                ),
                MarketType::Perp | MarketType::Spot => format!(
                    "{} {} does not support {} markets",
                    surface.label(),
                    operation.label(),
                    market_type_label(market_type)
                ),
            },
        }
    }
}

pub(crate) fn validate_surface_market_type(
    surface: OrderSurface,
    operation: OrderOperation,
    market_type: MarketType,
) -> Result<(), OrderCapabilityError> {
    if surface.allows_market_type(operation, market_type) {
        Ok(())
    } else {
        Err(OrderCapabilityError::UnsupportedMarketType {
            surface,
            operation,
            market_type,
        })
    }
}

pub(crate) fn place_order_task<F>(
    key: Zeroizing<String>,
    request: PlaceOrderRequest,
    map: F,
) -> Task<Message>
where
    F: FnOnce(Result<ExchangeResponse, String>) -> Message + Send + 'static,
{
    Task::perform(place_order_with_cloid(key, request), map)
}

pub(crate) fn cancel_order_task<F>(
    key: Zeroizing<String>,
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
    key: Zeroizing<String>,
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
    key: Zeroizing<String>,
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

impl OrderSurface {
    pub(crate) fn allows_market_type(
        self,
        operation: OrderOperation,
        market_type: MarketType,
    ) -> bool {
        match operation {
            OrderOperation::Cancel => true,
            OrderOperation::Modify => {
                matches!(self, Self::Move) || market_type != MarketType::Outcome
            }
            OrderOperation::UpdateLeverage => market_type == MarketType::Perp,
            OrderOperation::Place => match self {
                Self::Ticket | Self::Preset => true,
                Self::QuickOrder
                | Self::Hud
                | Self::ClosePosition
                | Self::Cluster
                | Self::Chase
                | Self::Twap => market_type != MarketType::Outcome,
                Self::ClusterClose | Self::Nuke => market_type == MarketType::Perp,
                Self::Move | Self::Cancel => true,
            },
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Ticket => "Ticket",
            Self::Preset => "Preset",
            Self::QuickOrder => "Quick order",
            Self::Hud => "HUD order",
            Self::ClosePosition => "Position close",
            Self::Cluster => "Wallet cluster",
            Self::ClusterClose => "Cluster close",
            Self::Nuke => "NUKE",
            Self::Chase => "Chase",
            Self::Twap => "TWAP",
            Self::Move => "Move order",
            Self::Cancel => "Cancel order",
        }
    }

    fn outcome_action_label(self, operation: OrderOperation) -> &'static str {
        match (self, operation) {
            (Self::QuickOrder, OrderOperation::Place) => "trading",
            (Self::Hud, OrderOperation::Place) => "HUD trading",
            (Self::ClosePosition, OrderOperation::Place) => "position closing",
            (Self::ClusterClose, OrderOperation::Place) => "cluster position closing",
            (Self::Chase, OrderOperation::Place) => "chase trading",
            (Self::Twap, OrderOperation::Place) => "TWAP trading",
            _ => operation.label(),
        }
    }

    pub(crate) fn orderability_context_label(self) -> &'static str {
        match self {
            Self::Ticket | Self::Preset | Self::Chase | Self::Twap => "Active",
            Self::Cluster => "Cluster",
            Self::QuickOrder | Self::Hud => "Chart",
            Self::ClosePosition | Self::ClusterClose | Self::Nuke => "Position",
            Self::Move | Self::Cancel => "Order",
        }
    }

    fn uses_connected_account_state(self) -> bool {
        matches!(
            self,
            Self::Ticket
                | Self::Preset
                | Self::QuickOrder
                | Self::Hud
                | Self::ClosePosition
                | Self::Nuke
                | Self::Chase
                | Self::Twap
        )
    }

    fn symbol_not_found_status_text(self, symbol_key: &str) -> String {
        match self {
            Self::QuickOrder
            | Self::Hud
            | Self::ClosePosition
            | Self::Cluster
            | Self::ClusterClose
            | Self::Nuke
            | Self::Move
            | Self::Cancel => {
                format!("Symbol '{symbol_key}' not found")
            }
            Self::Ticket | Self::Preset | Self::Chase | Self::Twap => {
                format!("Symbol '{symbol_key}' not found in exchange metadata")
            }
        }
    }
}

impl OrderOperation {
    fn label(self) -> &'static str {
        match self {
            Self::Place => "placement",
            Self::Cancel => "cancellation",
            Self::Modify => "modification",
            Self::UpdateLeverage => "leverage update",
        }
    }
}

fn market_type_label(market_type: MarketType) -> &'static str {
    match market_type {
        MarketType::Perp => "perpetual",
        MarketType::Spot => "spot",
        MarketType::Outcome => "outcome",
    }
}

fn allocate_one_shot_cloid_nonce_from(last_nonce_ms: &AtomicU64, now_ms: u64) -> u64 {
    let mut last = last_nonce_ms.load(Ordering::Relaxed);
    loop {
        let next = now_ms.max(last.saturating_add(1));
        match last_nonce_ms.compare_exchange_weak(last, next, Ordering::SeqCst, Ordering::Relaxed) {
            Ok(_) => return next,
            Err(observed) => last = observed,
        }
    }
}

fn next_one_shot_place_cloid(account_address: &str, order: &PreparedExchangeOrder) -> String {
    let nonce = allocate_one_shot_cloid_nonce_from(&LAST_ONE_SHOT_CLOID_NONCE_MS, now_ms());
    one_shot_place_cloid(account_address, nonce, order)
}

fn one_shot_place_cloid(
    account_address: &str,
    nonce: u64,
    order: &PreparedExchangeOrder,
) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(b"kerosene:one-shot-place");
    hasher.update(account_address.as_bytes());
    hasher.update(order.surface.cloid_tag().as_bytes());
    hasher.update(order.symbol_key.as_bytes());
    hasher.update(order.asset.to_be_bytes());
    hasher.update([u8::from(order.is_buy)]);
    hasher.update(order.price.as_bytes());
    hasher.update(order.size.as_bytes());
    hasher.update(order.order_kind.cloid_tag().as_bytes());
    hasher.update([u8::from(order.reduce_only)]);
    hasher.update(nonce.to_be_bytes());

    let digest = hasher.finalize();
    let mut cloid = String::with_capacity(34);
    cloid.push_str("0x");
    for byte in digest.iter().take(16) {
        let _ = write!(cloid, "{byte:02x}");
    }
    cloid
}

impl OrderSurface {
    fn cloid_tag(self) -> &'static str {
        match self {
            Self::Ticket => "ticket",
            Self::Preset => "preset",
            Self::QuickOrder => "quick",
            Self::Hud => "hud",
            Self::ClosePosition => "close",
            Self::Cluster => "cluster",
            Self::ClusterClose => "cluster_close",
            Self::Nuke => "nuke",
            Self::Chase => "chase",
            Self::Twap => "twap",
            Self::Move => "move",
            Self::Cancel => "cancel",
        }
    }
}

impl ExchangeOrderKind {
    fn cloid_tag(self) -> &'static str {
        match self {
            Self::Market => "market",
            Self::Limit => "limit",
            Self::LimitIoc => "limit_ioc",
        }
    }
}

impl TradingTerminal {
    fn validate_place_account_market_state(
        &self,
        surface: OrderSurface,
        market_type: MarketType,
    ) -> Result<(), String> {
        if market_type != MarketType::Perp || !surface.uses_connected_account_state() {
            return Ok(());
        }
        if self
            .connected_order_account_snapshot()
            .is_some_and(|(_, data)| !data.completeness.positions_actionable)
        {
            return Err(
                "Perpetual account state is incomplete; refresh account data before placing an order"
                    .to_string(),
            );
        }
        Ok(())
    }

    pub(crate) fn prepare_cancel_order(
        &self,
        intent: CancelIntent,
    ) -> Result<PreparedCancelOrder, String> {
        if let Some(sym) = self.exchange_symbol_for_key(&intent.symbol_key) {
            validate_surface_market_type(intent.surface, OrderOperation::Cancel, sym.market_type)
                .map_err(OrderCapabilityError::status_text)?;

            return Ok(PreparedCancelOrder {
                surface: intent.surface,
                symbol_key: sym.key.clone(),
                asset: sym.asset_index,
                oid: intent.oid,
                market_type: sym.market_type,
            });
        }

        // Open-order snapshots identify spot markets as "@{index}", except
        // for the established API-named PURR/USDC pair. Cancellation can
        // safely recover those deterministic asset ids while metadata is
        // unavailable. Keep this fallback cancellation-only: placement and
        // modification still require complete metadata for decimals,
        // orderability, and market-type validation.
        let Some(asset) = metadata_free_spot_cancel_asset(&intent.symbol_key) else {
            return Err(intent
                .surface
                .symbol_not_found_status_text(&intent.symbol_key));
        };
        validate_surface_market_type(intent.surface, OrderOperation::Cancel, MarketType::Spot)
            .map_err(OrderCapabilityError::status_text)?;

        Ok(PreparedCancelOrder {
            surface: intent.surface,
            symbol_key: intent.symbol_key,
            asset,
            oid: intent.oid,
            market_type: MarketType::Spot,
        })
    }

    pub(crate) fn prepare_modify_order(
        &self,
        intent: ModifyIntent,
    ) -> Result<PreparedModifyOrderResult, String> {
        let Some(sym) = self.exchange_symbol_for_key(&intent.symbol_key) else {
            return Err(intent
                .surface
                .symbol_not_found_status_text(&intent.symbol_key));
        };
        self.validate_exchange_symbol_orderable(sym, intent.surface.orderability_context_label())?;
        self.validate_spot_quantity_denomination(&sym.key, false)?;
        validate_surface_market_type(intent.surface, OrderOperation::Modify, sym.market_type)
            .map_err(OrderCapabilityError::status_text)?;

        let original_price = intent
            .original_price
            .trim()
            .parse::<f64>()
            .ok()
            .and_then(positive_finite_value)
            .ok_or_else(|| intent.invalid_price_message.to_string())?;
        let raw_size = intent
            .size
            .trim()
            .parse::<f64>()
            .ok()
            .and_then(finite_value)
            .filter(|size| *size > 1e-12)
            .ok_or_else(|| intent.invalid_size_message.to_string())?;
        if sym.market_type == MarketType::Outcome {
            self.validate_outcome_contract_size(raw_size)
                .map_err(|message| format!("Move failed: {message}"))?;
        }
        let reduce_only = if Self::market_type_is_spot_like(sym.market_type) {
            false
        } else {
            intent
                .reduce_only
                .ok_or_else(|| intent.reduce_only_missing_message.to_string())?
        };
        let new_price = finite_value(intent.new_price)
            .ok_or_else(|| intent.invalid_price_message.to_string())?;
        let is_spot_like = Self::market_type_is_spot_like(sym.market_type);
        let rounded = round_price(new_price, sym.sz_decimals, is_spot_like);
        let rounded = positive_finite_value(rounded)
            .ok_or_else(|| intent.invalid_price_message.to_string())?;
        let rounded_original = round_price(original_price, sym.sz_decimals, is_spot_like);
        if (rounded - rounded_original).abs() < 1e-12 {
            return Ok(PreparedModifyOrderResult::NoPriceChange);
        }

        validate_prepared_price(
            self,
            &sym.key,
            rounded,
            sym.market_type == MarketType::Outcome,
        )?;

        Ok(PreparedModifyOrderResult::Prepared(PreparedModifyOrder {
            surface: intent.surface,
            symbol_key: sym.key.clone(),
            oid: intent.oid,
            asset: sym.asset_index,
            is_buy: intent.is_buy,
            price: float_to_wire(rounded),
            size: float_to_wire(raw_size),
            reduce_only,
            market_type: sym.market_type,
        }))
    }

    pub(crate) fn prepare_place_order(
        &self,
        intent: PlaceIntent,
    ) -> Result<PreparedExchangeOrder, String> {
        let Some(sym) = self.exchange_symbol_for_key(&intent.symbol_key) else {
            return Err(intent
                .surface
                .symbol_not_found_status_text(&intent.symbol_key));
        };
        self.validate_exchange_symbol_orderable(sym, intent.surface.orderability_context_label())?;
        validate_surface_market_type(intent.surface, OrderOperation::Place, sym.market_type)
            .map_err(OrderCapabilityError::status_text)?;
        self.validate_place_account_market_state(intent.surface, sym.market_type)?;

        let symbol_key = sym.key.as_str();
        let sz_decimals = sym.sz_decimals;
        let is_outcome = sym.market_type == MarketType::Outcome;
        let is_spot_like = Self::market_type_is_spot_like(sym.market_type);
        let input_quantity_is_usd = matches!(
            &intent.quantity_source,
            QuantitySource::UserInput {
                denomination: QuantityDenomination::UsdNotional,
                ..
            }
        ) && !is_outcome;

        self.validate_spot_quantity_denomination(symbol_key, input_quantity_is_usd)?;

        let (raw_qty, quantity_uses_price) = match &intent.quantity_source {
            QuantitySource::UserInput {
                value,
                invalid_message,
                ..
            } => parse_positive_number(value)
                .map(|quantity| (quantity, input_quantity_is_usd))
                .ok_or_else(|| (*invalid_message).to_string()),
            QuantitySource::CoinSize {
                size,
                invalid_message,
                ..
            } => positive_finite_value(*size)
                .map(|quantity| (quantity, false))
                .ok_or_else(|| (*invalid_message).to_string()),
            QuantitySource::SpotPercentageBalance {
                available_balance,
                percentage,
                invalid_message,
                ..
            } => {
                if sym.market_type != MarketType::Spot
                    || !percentage.is_finite()
                    || *percentage <= 0.0
                    || *percentage > 100.0
                {
                    Err((*invalid_message).to_string())
                } else {
                    positive_finite_value(*available_balance)
                        .and_then(|balance| {
                            positive_finite_value(balance * (*percentage as f64 / 100.0))
                        })
                        .map(|quantity| (quantity, intent.is_buy))
                        .ok_or_else(|| (*invalid_message).to_string())
                }
            }
        }?;
        if is_outcome {
            self.validate_outcome_contract_size(raw_qty)?;
        }

        let (price, usd_size_reference_price) = match &intent.price_source {
            PriceSource::LimitInput {
                value,
                invalid_message,
            } => {
                let px =
                    parse_positive_number(value).ok_or_else(|| (*invalid_message).to_string())?;
                let rounded = round_price(px, sz_decimals, is_spot_like);
                let rounded =
                    positive_finite_value(rounded).ok_or_else(|| (*invalid_message).to_string())?;
                validate_prepared_price(self, symbol_key, rounded, is_outcome)?;
                (rounded, rounded)
            }
            PriceSource::MarketWithSlippage {
                invalid_message,
                usd_size_reference,
            } => {
                let Some(mid) = self.resolve_mid_for_symbol(symbol_key) else {
                    return Err(format!(
                        "No mid price for {} (tried {})",
                        self.display_name_for_symbol(symbol_key),
                        self.mid_candidates_for_symbol(symbol_key).join(", ")
                    ));
                };
                let rounded = if is_outcome {
                    let slipped =
                        slipped_market_price(mid, intent.is_buy, self.market_slippage_fraction());
                    let clamped = Self::clamp_outcome_market_price(slipped);
                    let rounded = round_price(clamped, sz_decimals, is_spot_like);
                    Self::clamp_outcome_market_price(rounded)
                } else {
                    rounded_market_price(
                        mid,
                        intent.is_buy,
                        self.market_slippage_fraction(),
                        sz_decimals,
                        is_spot_like,
                    )
                };
                let rounded = positive_finite_value(rounded).ok_or_else(|| {
                    invalid_message
                        .unwrap_or("Invalid market price")
                        .to_string()
                })?;
                validate_prepared_price(self, symbol_key, rounded, is_outcome)?;
                let usd_size_reference_price = match usd_size_reference {
                    MarketUsdSizeReference::ExecutionPrice => rounded,
                    MarketUsdSizeReference::Mid => mid,
                };
                (rounded, usd_size_reference_price)
            }
            PriceSource::ReferenceMid => {
                let Some(mid) = self.resolve_mid_for_symbol(symbol_key) else {
                    return Err(format!(
                        "No mid price for {} (tried {})",
                        self.display_name_for_symbol(symbol_key),
                        self.mid_candidates_for_symbol(symbol_key).join(", ")
                    ));
                };
                let rounded = round_price(mid, sz_decimals, is_spot_like);
                let rounded = positive_finite_value(rounded)
                    .ok_or_else(|| "Invalid reference price".to_string())?;
                validate_prepared_price(self, symbol_key, rounded, is_outcome)?;
                (rounded, rounded)
            }
        };

        let precision_invalid_message = match &intent.quantity_source {
            QuantitySource::UserInput {
                precision_invalid_message,
                ..
            } => *precision_invalid_message,
            QuantitySource::CoinSize {
                precision_invalid_message,
                ..
            } => *precision_invalid_message,
            QuantitySource::SpotPercentageBalance {
                precision_invalid_message,
                ..
            } => *precision_invalid_message,
        };
        let size_reference_price = if matches!(
            intent.quantity_source,
            QuantitySource::SpotPercentageBalance { .. }
        ) {
            price
        } else {
            usd_size_reference_price
        };
        let qty = order_size_from_quantity_input(
            raw_qty,
            size_reference_price,
            quantity_uses_price,
            sz_decimals,
        )
        .ok_or_else(|| precision_invalid_message.to_string())?;
        if is_outcome {
            self.validate_outcome_contract_size(qty)?;
        }

        let reduce_only = match intent.reduce_only_source {
            ReduceOnlySource::Form(reduce_only) => !is_spot_like && reduce_only,
            ReduceOnlySource::Fixed(reduce_only) => reduce_only,
        };

        Ok(PreparedExchangeOrder {
            surface: intent.surface,
            symbol_key: sym.key.clone(),
            asset: sym.asset_index,
            is_buy: intent.is_buy,
            price: float_to_wire(price),
            size: float_to_wire(qty),
            order_kind: intent.order_kind,
            reduce_only,
            market_type: sym.market_type,
        })
    }
}

/// Recover the only spot asset ids that are unambiguous without metadata.
/// PURR/USDC is the API-named universe index zero; every other supported form
/// must be the canonical indexed key.
fn metadata_free_spot_cancel_asset(key: &str) -> Option<u32> {
    if key == "PURR/USDC" {
        return Some(10_000);
    }

    let index = key.strip_prefix('@')?;
    if index.is_empty()
        || (index.len() > 1 && index.starts_with('0'))
        || !index.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    10_000u32.checked_add(index.parse::<u32>().ok()?)
}

fn validate_prepared_price(
    terminal: &TradingTerminal,
    symbol_key: &str,
    price: f64,
    is_outcome: bool,
) -> Result<(), String> {
    if is_outcome {
        TradingTerminal::validate_outcome_order_price(price)?;
    }
    terminal.validate_order_price_band(symbol_key, price)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{
        AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary,
        SpotClearinghouseState, UserFeeRates,
    };
    use crate::api::{ExchangeSymbol, OutcomeSymbolInfo};
    use crate::signing::ExchangeOrderKind;

    fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key.to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 7,
            collateral_token: None,
            sz_decimals: 4,
            max_leverage: 50,
            only_isolated: false,
            market_type,
            outcome: None,
        }
    }

    fn outcome_symbol(key: &str) -> ExchangeSymbol {
        ExchangeSymbol {
            sz_decimals: 0,
            market_type: MarketType::Outcome,
            outcome: Some(OutcomeSymbolInfo {
                outcome_id: 65,
                question_id: Some(12),
                question_name: Some("Recurring".to_string()),
                question_description: None,
                question_class: Some("priceBucket".to_string()),
                question_underlying: Some("BTC".to_string()),
                question_expiry: Some("20260520-0600".to_string()),
                question_price_thresholds: Vec::new(),
                question_period: None,
                question_named_outcomes: Vec::new(),
                question_settled_named_outcomes: Vec::new(),
                question_fallback_outcome: None,
                bucket_index: Some(0),
                is_question_fallback: false,
                side_index: 0,
                side_name: "Yes".to_string(),
                outcome_name: "Recurring Named Outcome".to_string(),
                description: "index:0".to_string(),
                class: None,
                underlying: None,
                expiry: None,
                target_price: None,
                period: None,
                quote_symbol: "USDC".to_string(),
                quote_token_index: Some(crate::api::USDC_TOKEN_INDEX),
                encoding: 650,
            }),
            ..symbol(key, MarketType::Outcome)
        }
    }

    fn incomplete_perp_account_data() -> AccountData {
        let mut completeness = AccountDataCompleteness::default();
        completeness.positions_complete = false;
        completeness.positions_actionable = false;
        AccountData {
            fetch_scope: Default::default(),
            request_weight_estimate: 0,
            account_abstraction: Default::default(),
            clearinghouse: ClearinghouseState {
                margin_summary: MarginSummary {
                    account_value: "0".to_string(),
                    total_ntl_pos: "0".to_string(),
                    total_margin_used: "0".to_string(),
                },
                cross_margin_summary: None,
                cross_maintenance_margin_used: None,
                withdrawable: "0".to_string(),
                asset_positions: Vec::new(),
            },
            clearinghouses_by_dex: std::collections::HashMap::new(),
            spot: SpotClearinghouseState {
                balances: Vec::new(),
                portfolio_margin_enabled: false,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            open_orders: Vec::new(),
            fills: Vec::new(),
            funding_history: Vec::new(),
            fee_rates: UserFeeRates::default(),
            completeness,
            fetched_at_ms: TradingTerminal::now_ms(),
        }
    }

    #[test]
    fn failed_perp_bootstrap_blocks_perp_placement_but_not_spot_placement_state() {
        const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.connected_address = Some(ADDRESS.to_string());
        terminal.set_account_data_for_address_for_test(ADDRESS, incomplete_perp_account_data());

        assert_eq!(
            terminal.validate_place_account_market_state(OrderSurface::Ticket, MarketType::Perp),
            Err(
                "Perpetual account state is incomplete; refresh account data before placing an order"
                    .to_string()
            )
        );
        assert_eq!(
            terminal.validate_place_account_market_state(OrderSurface::Ticket, MarketType::Spot),
            Ok(())
        );
        assert_eq!(
            terminal.validate_place_account_market_state(OrderSurface::Cluster, MarketType::Perp),
            Ok(()),
            "cluster legs validate their own member snapshots"
        );
    }

    fn ticket_limit_intent(symbol_key: &str) -> PlaceIntent {
        PlaceIntent {
            surface: OrderSurface::Ticket,
            symbol_key: symbol_key.to_string(),
            is_buy: true,
            order_kind: ExchangeOrderKind::Limit,
            price_source: PriceSource::LimitInput {
                value: "100.123456".to_string(),
                invalid_message: "Invalid price",
            },
            quantity_source: QuantitySource::UserInput {
                value: "250.5".to_string(),
                denomination: QuantityDenomination::UsdNotional,
                invalid_message: "Invalid quantity",
                precision_invalid_message: "Invalid quantity for asset precision",
            },
            reduce_only_source: ReduceOnlySource::Form(true),
        }
    }

    fn market_usd_intent(
        surface: OrderSurface,
        reference: MarketUsdSizeReference,
        is_buy: bool,
    ) -> PlaceIntent {
        PlaceIntent {
            surface,
            symbol_key: "BTC".to_string(),
            is_buy,
            order_kind: ExchangeOrderKind::Market,
            price_source: PriceSource::MarketWithSlippage {
                invalid_message: Some("Invalid market price"),
                usd_size_reference: reference,
            },
            quantity_source: QuantitySource::UserInput {
                value: "250".to_string(),
                denomination: QuantityDenomination::UsdNotional,
                invalid_message: "Invalid quantity",
                precision_invalid_message: "Invalid quantity for asset precision",
            },
            reduce_only_source: ReduceOnlySource::Form(false),
        }
    }

    #[test]
    fn place_intent_debug_redacts_symbol_price_and_quantity() {
        let intent = PlaceIntent {
            surface: OrderSurface::Ticket,
            symbol_key: "SECRETCOIN".to_string(),
            is_buy: true,
            order_kind: ExchangeOrderKind::Limit,
            price_source: PriceSource::LimitInput {
                value: "price-secret".to_string(),
                invalid_message: "Invalid price",
            },
            quantity_source: QuantitySource::UserInput {
                value: "quantity-secret".to_string(),
                denomination: QuantityDenomination::UsdNotional,
                invalid_message: "Invalid quantity",
                precision_invalid_message: "Invalid quantity for asset precision",
            },
            reduce_only_source: ReduceOnlySource::Form(true),
        };

        let rendered = format!("{intent:?}");

        assert!(rendered.contains("symbol_key: <redacted>"));
        assert!(rendered.contains("value: <redacted>"));
        assert!(rendered.contains("denomination: UsdNotional"));
        for secret in ["SECRETCOIN", "price-secret", "quantity-secret"] {
            assert!(!rendered.contains(secret), "{secret} leaked in {rendered}");
        }
    }

    #[test]
    fn prepared_order_debug_redacts_symbols_prices_sizes_and_ids() {
        let cancel = PreparedCancelOrder {
            surface: OrderSurface::Cancel,
            symbol_key: "CANCELSECRET".to_string(),
            asset: 7,
            oid: 123456789,
            market_type: MarketType::Perp,
        };
        let modify = PreparedModifyOrder {
            surface: OrderSurface::Move,
            symbol_key: "MODIFYSECRET".to_string(),
            oid: 987654321,
            asset: 8,
            is_buy: false,
            price: "modify-price-secret".to_string(),
            size: "modify-size-secret".to_string(),
            reduce_only: true,
            market_type: MarketType::Perp,
        };
        let place = PreparedExchangeOrder {
            surface: OrderSurface::Ticket,
            symbol_key: "PLACESECRET".to_string(),
            asset: 9,
            is_buy: true,
            price: "place-price-secret".to_string(),
            size: "place-size-secret".to_string(),
            order_kind: ExchangeOrderKind::Limit,
            reduce_only: false,
            market_type: MarketType::Perp,
        };

        let rendered = format!("{cancel:?} {modify:?} {place:?}");

        assert!(rendered.contains("symbol_key: <redacted>"));
        assert!(rendered.contains("oid: <redacted>"));
        assert!(rendered.contains("price: <redacted>"));
        assert!(rendered.contains("size: <redacted>"));
        for secret in [
            "CANCELSECRET",
            "MODIFYSECRET",
            "PLACESECRET",
            "123456789",
            "987654321",
            "modify-price-secret",
            "modify-size-secret",
            "place-price-secret",
            "place-size-secret",
        ] {
            assert!(!rendered.contains(secret), "{secret} leaked in {rendered}");
        }
    }

    fn move_modify_intent(symbol_key: &str) -> ModifyIntent {
        ModifyIntent {
            surface: OrderSurface::Move,
            symbol_key: symbol_key.to_string(),
            oid: 42,
            is_buy: true,
            new_price: 101.0,
            original_price: "100".to_string(),
            size: "0.25".to_string(),
            invalid_size_message: "Move failed: open order has invalid size",
            reduce_only: Some(false),
            reduce_only_missing_message: concat!(
                "Move failed: open order reduce-only metadata is unavailable; ",
                "refresh account data before moving this order"
            ),
            invalid_price_message: "Move failed: open order has invalid price",
        }
    }

    #[test]
    fn ticket_and_presets_can_place_outcome_orders() {
        for surface in [OrderSurface::Ticket, OrderSurface::Preset] {
            assert!(surface.allows_market_type(OrderOperation::Place, MarketType::Outcome));
        }
    }

    #[test]
    fn chart_position_and_strategy_surfaces_reject_outcome_placements() {
        for surface in [
            OrderSurface::QuickOrder,
            OrderSurface::Hud,
            OrderSurface::ClosePosition,
            OrderSurface::Cluster,
            OrderSurface::Chase,
            OrderSurface::Twap,
        ] {
            assert!(!surface.allows_market_type(OrderOperation::Place, MarketType::Outcome));
        }
    }

    #[test]
    fn cluster_surfaces_match_perp_and_spot_but_not_outcome() {
        // Standard cluster orders mirror the ticket across wallets for perp and
        // spot, but must exclude prediction (Outcome) markets like every other
        // secondary surface.
        assert!(OrderSurface::Cluster.allows_market_type(OrderOperation::Place, MarketType::Perp));
        assert!(OrderSurface::Cluster.allows_market_type(OrderOperation::Place, MarketType::Spot));
        assert!(
            !OrderSurface::Cluster.allows_market_type(OrderOperation::Place, MarketType::Outcome)
        );
        // Cluster closes are reduce-only perp closes only.
        assert!(
            OrderSurface::ClusterClose.allows_market_type(OrderOperation::Place, MarketType::Perp)
        );
        assert!(
            !OrderSurface::ClusterClose.allows_market_type(OrderOperation::Place, MarketType::Spot)
        );
        assert!(
            !OrderSurface::ClusterClose
                .allows_market_type(OrderOperation::Place, MarketType::Outcome)
        );
    }

    #[test]
    fn move_and_cancel_keep_outcome_support_for_existing_orders() {
        assert!(OrderSurface::Move.allows_market_type(OrderOperation::Modify, MarketType::Outcome));
        assert!(
            OrderSurface::Cancel.allows_market_type(OrderOperation::Cancel, MarketType::Outcome)
        );
    }

    #[test]
    fn nuke_only_places_perp_orders() {
        assert!(OrderSurface::Nuke.allows_market_type(OrderOperation::Place, MarketType::Perp));
        assert!(!OrderSurface::Nuke.allows_market_type(OrderOperation::Place, MarketType::Spot));
        assert!(!OrderSurface::Nuke.allows_market_type(OrderOperation::Place, MarketType::Outcome));
    }

    #[test]
    fn unsupported_outcome_message_matches_existing_surface_text() {
        let error = validate_surface_market_type(
            OrderSurface::Hud,
            OrderOperation::Place,
            MarketType::Outcome,
        )
        .unwrap_err();

        assert_eq!(
            error.status_text(),
            "Outcome HUD trading is not available from this control; use the main order ticket"
        );
    }

    #[test]
    fn leverage_updates_are_perp_only() {
        assert!(
            OrderSurface::Ticket
                .allows_market_type(OrderOperation::UpdateLeverage, MarketType::Perp)
        );
        assert!(
            !OrderSurface::Ticket
                .allows_market_type(OrderOperation::UpdateLeverage, MarketType::Spot)
        );
    }

    #[test]
    fn prepare_ticket_limit_order_converts_usd_size_and_reduce_only_for_perps() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.all_mids.insert("BTC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());

        let prepared = terminal
            .prepare_place_order(ticket_limit_intent("BTC"))
            .expect("valid prepared order");

        assert_eq!(
            prepared,
            PreparedExchangeOrder {
                surface: OrderSurface::Ticket,
                symbol_key: "BTC".to_string(),
                asset: 7,
                is_buy: true,
                price: "100.12".to_string(),
                size: "2.5019".to_string(),
                order_kind: ExchangeOrderKind::Limit,
                reduce_only: true,
                market_type: MarketType::Perp,
            }
        );
    }

    #[test]
    fn prepare_quick_market_usd_order_sizes_from_mid_not_slipped_execution_price() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.market_slippage_pct = 5.0;
        terminal.all_mids.insert("BTC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());

        let buy = terminal
            .prepare_place_order(market_usd_intent(
                OrderSurface::QuickOrder,
                MarketUsdSizeReference::Mid,
                true,
            ))
            .expect("valid quick market buy");
        let sell = terminal
            .prepare_place_order(market_usd_intent(
                OrderSurface::QuickOrder,
                MarketUsdSizeReference::Mid,
                false,
            ))
            .expect("valid quick market sell");

        assert_eq!(buy.price, "105");
        assert_eq!(sell.price, "95");
        assert_eq!(buy.size, "2.5");
        assert_eq!(sell.size, "2.5");
    }

    #[test]
    fn prepare_ticket_market_usd_order_can_size_from_slipped_execution_price() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.market_slippage_pct = 5.0;
        terminal.all_mids.insert("BTC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());

        let prepared = terminal
            .prepare_place_order(market_usd_intent(
                OrderSurface::Ticket,
                MarketUsdSizeReference::ExecutionPrice,
                true,
            ))
            .expect("valid ticket market buy");

        assert_eq!(prepared.price, "105");
        assert_eq!(prepared.size, "2.3809");
    }

    #[test]
    fn spot_percentage_sell_uses_exact_base_balance_at_sub_cent_prices() {
        let (mut terminal, _) = TradingTerminal::boot();
        let mut spot = symbol("@7", MarketType::Spot);
        spot.ticker = "LOW".to_string();
        spot.display_name = Some("LOW/USDC".to_string());
        spot.collateral_token = Some(crate::api::USDC_TOKEN_INDEX);
        terminal.exchange_symbols = vec![spot];
        terminal.all_mids.insert("@7".to_string(), 0.00035);
        terminal
            .all_mids_updated_at_ms
            .insert("@7".to_string(), TradingTerminal::now_ms());

        let prepared = terminal
            .prepare_place_order(PlaceIntent {
                surface: OrderSurface::Ticket,
                symbol_key: "@7".to_string(),
                is_buy: false,
                order_kind: ExchangeOrderKind::Limit,
                price_source: PriceSource::LimitInput {
                    value: "0.00035".to_string(),
                    invalid_message: "Invalid price",
                },
                quantity_source: QuantitySource::SpotPercentageBalance {
                    available_balance: 100.0,
                    percentage: 100.0,
                    invalid_message: "Invalid spot percentage balance",
                    precision_invalid_message: "Invalid spot percentage size",
                },
                reduce_only_source: ReduceOnlySource::Form(false),
            })
            .expect("exact spot percentage sell should prepare");

        let size: f64 = prepared.size.parse().expect("wire size");
        assert!(size <= 100.0);
        assert_eq!(size, 100.0);
    }

    #[test]
    fn spot_percentage_buy_cannot_spend_more_than_fractional_quote_balance() {
        let (mut terminal, _) = TradingTerminal::boot();
        let mut spot = symbol("@7", MarketType::Spot);
        spot.ticker = "LOW".to_string();
        spot.display_name = Some("LOW/USDC".to_string());
        spot.collateral_token = Some(crate::api::USDC_TOKEN_INDEX);
        terminal.exchange_symbols = vec![spot];
        terminal.market_slippage_pct = 5.0;
        terminal.all_mids.insert("@7".to_string(), 0.35);
        terminal
            .all_mids_updated_at_ms
            .insert("@7".to_string(), TradingTerminal::now_ms());
        let quote_balance = 1.000_001;

        let prepared = terminal
            .prepare_place_order(PlaceIntent {
                surface: OrderSurface::QuickOrder,
                symbol_key: "@7".to_string(),
                is_buy: true,
                order_kind: ExchangeOrderKind::Market,
                price_source: PriceSource::MarketWithSlippage {
                    invalid_message: Some("Invalid market price"),
                    usd_size_reference: MarketUsdSizeReference::Mid,
                },
                quantity_source: QuantitySource::SpotPercentageBalance {
                    available_balance: quote_balance,
                    percentage: 100.0,
                    invalid_message: "Invalid spot percentage balance",
                    precision_invalid_message: "Invalid spot percentage size",
                },
                reduce_only_source: ReduceOnlySource::Form(false),
            })
            .expect("exact spot percentage buy should prepare");

        let size: f64 = prepared.size.parse().expect("wire size");
        let price: f64 = prepared.price.parse().expect("wire price");
        assert!(size * price <= quote_balance + 1e-12);
    }

    #[test]
    fn prepare_limit_order_rejects_prices_that_round_to_zero() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.all_mids.insert("BTC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
        let mut intent = ticket_limit_intent("BTC");
        intent.price_source = PriceSource::LimitInput {
            value: "0.0000001".to_string(),
            invalid_message: "Invalid price",
        };

        let error = terminal.prepare_place_order(intent).unwrap_err();

        assert_eq!(error, "Invalid price");
    }

    #[test]
    fn prepare_market_order_rejects_prices_that_round_to_zero() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.all_mids.insert("BTC".to_string(), 0.0000001);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());

        let error = terminal
            .prepare_place_order(market_usd_intent(
                OrderSurface::QuickOrder,
                MarketUsdSizeReference::Mid,
                true,
            ))
            .unwrap_err();

        assert_eq!(error, "Invalid market price");
    }

    #[test]
    fn prepare_market_order_missing_mid_error_leads_with_outcome_display_label() {
        let (mut terminal, _) = TradingTerminal::boot();
        let sym = outcome_symbol("#650");
        let label = TradingTerminal::exchange_symbol_display_name(&sym);
        terminal.exchange_symbols = vec![sym];
        terminal.all_mids.clear();
        terminal.all_mids_updated_at_ms.clear();
        let mut intent = ticket_limit_intent("#650");
        intent.order_kind = ExchangeOrderKind::Market;
        intent.price_source = PriceSource::MarketWithSlippage {
            invalid_message: Some("Invalid market price"),
            usd_size_reference: MarketUsdSizeReference::Mid,
        };
        intent.quantity_source = QuantitySource::UserInput {
            value: "3".to_string(),
            denomination: QuantityDenomination::Coin,
            invalid_message: "Invalid quantity",
            precision_invalid_message: "Invalid quantity for asset precision",
        };

        let error = terminal.prepare_place_order(intent).unwrap_err();

        assert!(error.starts_with(&format!("No mid price for {label}")));
        assert!(error.contains("(tried #650"));
    }

    #[test]
    fn prepare_ticket_outcome_order_forces_coin_quantity_and_clears_reduce_only() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![outcome_symbol("#650")];
        terminal.all_mids.insert("#650".to_string(), 0.42);
        terminal
            .all_mids_updated_at_ms
            .insert("#650".to_string(), TradingTerminal::now_ms());
        let mut intent = ticket_limit_intent("#650");
        intent.price_source = PriceSource::LimitInput {
            value: "0.421234".to_string(),
            invalid_message: "Invalid price",
        };
        intent.quantity_source = QuantitySource::UserInput {
            value: "3".to_string(),
            denomination: QuantityDenomination::UsdNotional,
            invalid_message: "Invalid quantity",
            precision_invalid_message: "Invalid quantity for asset precision",
        };

        let prepared = terminal
            .prepare_place_order(intent)
            .expect("valid outcome order");

        assert_eq!(prepared.price, "0.42123");
        assert_eq!(prepared.size, "3");
        assert!(!prepared.reduce_only);
        assert_eq!(prepared.market_type, MarketType::Outcome);
    }

    #[test]
    fn prepare_close_position_limit_uses_reference_mid_and_fixed_reduce_only() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.all_mids.insert("BTC".to_string(), 100.123456);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());

        let prepared = terminal
            .prepare_place_order(PlaceIntent {
                surface: OrderSurface::ClosePosition,
                symbol_key: "BTC".to_string(),
                is_buy: false,
                order_kind: ExchangeOrderKind::Limit,
                price_source: PriceSource::ReferenceMid,
                quantity_source: QuantitySource::CoinSize {
                    size: 1.239,
                    invalid_message: "Position size is invalid",
                    precision_invalid_message: "Position size is invalid",
                },
                reduce_only_source: ReduceOnlySource::Fixed(true),
            })
            .expect("valid close-position order");

        assert_eq!(prepared.price, "100.12");
        assert_eq!(prepared.size, "1.239");
        assert!(prepared.reduce_only);
    }

    #[test]
    fn prepare_close_position_rejects_outcomes_through_capability_policy() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![outcome_symbol("#650")];
        terminal.all_mids.insert("#650".to_string(), 0.42);
        terminal
            .all_mids_updated_at_ms
            .insert("#650".to_string(), TradingTerminal::now_ms());

        let error = terminal
            .prepare_place_order(PlaceIntent {
                surface: OrderSurface::ClosePosition,
                symbol_key: "#650".to_string(),
                is_buy: false,
                order_kind: ExchangeOrderKind::Limit,
                price_source: PriceSource::ReferenceMid,
                quantity_source: QuantitySource::CoinSize {
                    size: 1.0,
                    invalid_message: "Position size is invalid",
                    precision_invalid_message: "Position size is invalid",
                },
                reduce_only_source: ReduceOnlySource::Fixed(true),
            })
            .unwrap_err();

        assert_eq!(
            error,
            "Outcome position closing is not available from this control; use the main order ticket"
        );
    }

    fn purr_spot_symbol() -> ExchangeSymbol {
        ExchangeSymbol {
            ticker: "PURR".to_string(),
            category: "spot".to_string(),
            display_name: Some("PURR/USDC".to_string()),
            asset_index: 10_000,
            ..symbol("PURR/USDC", MarketType::Spot)
        }
    }

    #[test]
    fn prepare_cancel_order_accepts_api_named_spot_pair_and_legacy_indexed_key() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![purr_spot_symbol()];

        // Open orders report the API coin name ("PURR/USDC"), which must
        // resolve so resting spot orders can be canceled from the app.
        let prepared = terminal
            .prepare_cancel_order(CancelIntent {
                surface: OrderSurface::Cancel,
                symbol_key: "PURR/USDC".to_string(),
                oid: 42,
            })
            .expect("cancel by API coin name");
        assert_eq!(prepared.symbol_key, "PURR/USDC");
        assert_eq!(prepared.asset, 10_000);
        assert_eq!(prepared.market_type, MarketType::Spot);

        // State saved before the pair was re-keyed may still send "@0".
        let prepared = terminal
            .prepare_cancel_order(CancelIntent {
                surface: OrderSurface::Cancel,
                symbol_key: "@0".to_string(),
                oid: 42,
            })
            .expect("cancel by legacy indexed key");
        assert_eq!(prepared.symbol_key, "PURR/USDC");
        assert_eq!(prepared.asset, 10_000);
    }

    #[test]
    fn prepare_cancel_order_derives_indexed_spot_asset_without_metadata() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols.clear();

        let prepared = terminal
            .prepare_cancel_order(CancelIntent {
                surface: OrderSurface::Cancel,
                symbol_key: "@107".to_string(),
                oid: 42,
            })
            .expect("indexed spot cancellation should not depend on metadata");

        assert_eq!(prepared.symbol_key, "@107");
        assert_eq!(prepared.asset, 10_107);
        assert_eq!(prepared.oid, 42);
        assert_eq!(prepared.market_type, MarketType::Spot);
    }

    #[test]
    fn prepare_cancel_order_recovers_canonical_purr_without_metadata() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols.clear();

        let prepared = terminal
            .prepare_cancel_order(CancelIntent {
                surface: OrderSurface::Cancel,
                symbol_key: "PURR/USDC".to_string(),
                oid: 42,
            })
            .expect("canonical PURR cancellation should not depend on metadata");

        assert_eq!(prepared.symbol_key, "PURR/USDC");
        assert_eq!(prepared.asset, 10_000);
        assert_eq!(prepared.oid, 42);
        assert_eq!(prepared.market_type, MarketType::Spot);
    }

    #[test]
    fn prepare_cancel_order_rejects_noncanonical_or_overflowing_spot_keys() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols.clear();

        for key in [
            "@",
            "@-1",
            "@+1",
            "@01",
            "@1 ",
            "@1x",
            "@@1",
            "@4294967295",
            "HYPE/USDC",
            "purr/USDC",
        ] {
            let error = terminal
                .prepare_cancel_order(CancelIntent {
                    surface: OrderSurface::Cancel,
                    symbol_key: key.to_string(),
                    oid: 42,
                })
                .expect_err("invalid metadata-free key must fail closed");

            assert_eq!(error, format!("Symbol '{key}' not found"));
        }
    }

    #[test]
    fn metadata_free_indexed_spot_fallback_is_not_used_for_placement() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols.clear();

        let error = terminal
            .prepare_place_order(PlaceIntent {
                surface: OrderSurface::Ticket,
                symbol_key: "@107".to_string(),
                is_buy: true,
                order_kind: ExchangeOrderKind::Market,
                price_source: PriceSource::ReferenceMid,
                quantity_source: QuantitySource::CoinSize {
                    size: 1.0,
                    invalid_message: "Invalid quantity",
                    precision_invalid_message: "Invalid quantity for asset precision",
                },
                reduce_only_source: ReduceOnlySource::Form(false),
            })
            .expect_err("placement must still require exchange metadata");

        assert_eq!(error, "Symbol '@107' not found in exchange metadata");
    }

    #[test]
    fn spot_market_order_rejects_fresh_same_ticker_perp_mid() {
        let (mut terminal, _) = TradingTerminal::boot();
        let mut spot = symbol("@107", MarketType::Spot);
        spot.ticker = "HYPE".to_string();
        spot.display_name = Some("HYPE/USDC".to_string());
        spot.asset_index = 10_107;
        terminal.exchange_symbols = vec![symbol("HYPE", MarketType::Perp), spot];
        terminal.all_mids.insert("HYPE".to_string(), 40.0);
        terminal
            .all_mids_updated_at_ms
            .insert("HYPE".to_string(), TradingTerminal::now_ms());

        let error = terminal
            .prepare_place_order(PlaceIntent {
                surface: OrderSurface::Ticket,
                symbol_key: "@107".to_string(),
                is_buy: true,
                order_kind: ExchangeOrderKind::Market,
                price_source: PriceSource::MarketWithSlippage {
                    invalid_message: Some("Invalid market price"),
                    usd_size_reference: MarketUsdSizeReference::Mid,
                },
                quantity_source: QuantitySource::UserInput {
                    value: "100".to_string(),
                    denomination: QuantityDenomination::UsdNotional,
                    invalid_message: "Invalid quantity",
                    precision_invalid_message: "Invalid quantity for asset precision",
                },
                reduce_only_source: ReduceOnlySource::Form(false),
            })
            .expect_err("spot orders must not use a perpetual mid");

        assert_eq!(error, "No mid price for HYPE/USDC (tried @107)");
    }

    #[test]
    fn non_usd_quoted_spot_rejects_all_placement_denominations() {
        let (mut terminal, _) = TradingTerminal::boot();
        let mut spot = symbol("@55", MarketType::Spot);
        spot.ticker = "UETH".to_string();
        spot.display_name = Some("UETH/UBTC".to_string());
        spot.asset_index = 10_055;
        terminal.exchange_symbols = vec![spot];
        terminal.all_mids.insert("@55".to_string(), 0.05);
        terminal
            .all_mids_updated_at_ms
            .insert("@55".to_string(), TradingTerminal::now_ms());

        let usd_error = terminal
            .prepare_place_order(PlaceIntent {
                surface: OrderSurface::Ticket,
                symbol_key: "@55".to_string(),
                is_buy: true,
                order_kind: ExchangeOrderKind::Market,
                price_source: PriceSource::MarketWithSlippage {
                    invalid_message: Some("Invalid market price"),
                    usd_size_reference: MarketUsdSizeReference::Mid,
                },
                quantity_source: QuantitySource::UserInput {
                    value: "100".to_string(),
                    denomination: QuantityDenomination::UsdNotional,
                    invalid_message: "Invalid quantity",
                    precision_invalid_message: "Invalid quantity for asset precision",
                },
                reduce_only_source: ReduceOnlySource::Form(false),
            })
            .expect_err("a crypto-quoted pair has no safe USD conversion");

        assert_eq!(
            usd_error,
            "Spot trading is unavailable for UETH/UBTC because quote-token USD valuation and accounting are not verified"
        );

        let coin_error = terminal
            .prepare_place_order(PlaceIntent {
                surface: OrderSurface::Ticket,
                symbol_key: "@55".to_string(),
                is_buy: true,
                order_kind: ExchangeOrderKind::Market,
                price_source: PriceSource::MarketWithSlippage {
                    invalid_message: Some("Invalid market price"),
                    usd_size_reference: MarketUsdSizeReference::Mid,
                },
                quantity_source: QuantitySource::CoinSize {
                    size: 1.2345,
                    invalid_message: "Invalid quantity",
                    precision_invalid_message: "Invalid quantity for asset precision",
                },
                reduce_only_source: ReduceOnlySource::Form(false),
            })
            .expect_err("coin size must not bypass unverified quote accounting");

        assert_eq!(coin_error, usd_error);
    }

    #[test]
    fn non_usd_quoted_spot_can_be_cancelled_but_not_modified() {
        let (mut terminal, _) = TradingTerminal::boot();
        let mut spot = symbol("@55", MarketType::Spot);
        spot.ticker = "UETH".to_string();
        spot.display_name = Some("UETH/UBTC".to_string());
        spot.asset_index = 10_055;
        spot.collateral_token = Some(221);
        terminal.exchange_symbols = vec![spot];
        terminal.all_mids.insert("@55".to_string(), 0.05);
        terminal
            .all_mids_updated_at_ms
            .insert("@55".to_string(), TradingTerminal::now_ms());

        let modify_error = terminal
            .prepare_modify_order(ModifyIntent {
                surface: OrderSurface::Move,
                symbol_key: "@55".to_string(),
                oid: 42,
                is_buy: true,
                new_price: 0.051,
                original_price: "0.05".to_string(),
                size: "1".to_string(),
                invalid_size_message: "Invalid size",
                reduce_only: None,
                reduce_only_missing_message: "Missing reduce-only",
                invalid_price_message: "Invalid price",
            })
            .expect_err("unverified quote accounting must block repricing");
        assert!(modify_error.contains("quote-token USD valuation and accounting"));

        let cancel = terminal
            .prepare_cancel_order(CancelIntent {
                surface: OrderSurface::Cancel,
                symbol_key: "@55".to_string(),
                oid: 42,
            })
            .expect("safety cancellation must remain available");
        assert_eq!(cancel.market_type, MarketType::Spot);
    }

    #[test]
    fn prepare_modify_order_accepts_legacy_indexed_key_for_api_named_spot_pair() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![purr_spot_symbol()];
        terminal.all_mids.insert("PURR/USDC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("PURR/USDC".to_string(), TradingTerminal::now_ms());

        let prepared = terminal
            .prepare_modify_order(move_modify_intent("@0"))
            .expect("modify by legacy indexed key");

        match prepared {
            PreparedModifyOrderResult::Prepared(prepared) => {
                assert_eq!(prepared.symbol_key, "PURR/USDC");
                assert_eq!(prepared.asset, 10_000);
            }
            PreparedModifyOrderResult::NoPriceChange => panic!("expected prepared order"),
        }
    }

    #[test]
    fn prepare_cancel_order_allows_existing_outcome_orders() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![outcome_symbol("#650")];

        let prepared = terminal
            .prepare_cancel_order(CancelIntent {
                surface: OrderSurface::Cancel,
                symbol_key: "#650".to_string(),
                oid: 42,
            })
            .expect("cancel should be prepared");

        assert_eq!(prepared.symbol_key, "#650");
        assert_eq!(prepared.asset, 7);
        assert_eq!(prepared.oid, 42);
        assert_eq!(prepared.market_type, MarketType::Outcome);
    }

    #[test]
    fn prepare_move_modify_order_rounds_price_and_preserves_known_reduce_only() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.all_mids.insert("BTC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());

        let prepared = terminal
            .prepare_modify_order(move_modify_intent("BTC"))
            .expect("valid modify order");

        assert_eq!(
            prepared,
            PreparedModifyOrderResult::Prepared(PreparedModifyOrder {
                surface: OrderSurface::Move,
                symbol_key: "BTC".to_string(),
                oid: 42,
                asset: 7,
                is_buy: true,
                price: "101".to_string(),
                size: "0.25".to_string(),
                reduce_only: false,
                market_type: MarketType::Perp,
            })
        );
    }

    #[test]
    fn prepare_move_modify_order_returns_noop_when_rounded_price_is_unchanged() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.all_mids.insert("BTC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
        let mut intent = move_modify_intent("BTC");
        intent.new_price = 100.001;

        let prepared = terminal
            .prepare_modify_order(intent)
            .expect("valid no-op modify input");

        assert_eq!(prepared, PreparedModifyOrderResult::NoPriceChange);
    }

    #[test]
    fn prepare_move_modify_order_validates_size_price_and_reduce_only_metadata() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.all_mids.insert("BTC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());

        let mut invalid_size = move_modify_intent("BTC");
        invalid_size.size = "0".to_string();
        assert_eq!(
            terminal.prepare_modify_order(invalid_size).unwrap_err(),
            "Move failed: open order has invalid size"
        );

        let mut invalid_price = move_modify_intent("BTC");
        invalid_price.original_price = "0".to_string();
        assert_eq!(
            terminal.prepare_modify_order(invalid_price).unwrap_err(),
            "Move failed: open order has invalid price"
        );

        let mut missing_reduce_only = move_modify_intent("BTC");
        missing_reduce_only.reduce_only = None;
        assert!(
            terminal
                .prepare_modify_order(missing_reduce_only)
                .unwrap_err()
                .contains("reduce-only metadata is unavailable")
        );
    }

    #[test]
    fn prepare_move_modify_order_clears_missing_reduce_only_for_spot() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![purr_spot_symbol()];
        terminal.all_mids.insert("PURR/USDC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("PURR/USDC".to_string(), TradingTerminal::now_ms());
        let mut intent = move_modify_intent("PURR/USDC");
        intent.reduce_only = None;

        let prepared = terminal
            .prepare_modify_order(intent)
            .expect("spot modify order should not need reduce-only metadata");

        match prepared {
            PreparedModifyOrderResult::Prepared(prepared) => assert!(!prepared.reduce_only),
            PreparedModifyOrderResult::NoPriceChange => panic!("expected prepared order"),
        }
    }

    #[test]
    fn prepare_move_modify_order_keeps_outcome_contract_validation() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![outcome_symbol("#650")];
        terminal.all_mids.insert("#650".to_string(), 0.42);
        terminal
            .all_mids_updated_at_ms
            .insert("#650".to_string(), TradingTerminal::now_ms());
        let mut intent = move_modify_intent("#650");
        intent.original_price = "0.42".to_string();
        intent.new_price = 0.43;
        intent.size = "0.25".to_string();
        intent.reduce_only = None;

        let error = terminal.prepare_modify_order(intent).unwrap_err();

        assert!(error.contains("whole-contract sizes"));
    }

    #[test]
    fn one_shot_place_cloid_is_stable_128_bit_hex_for_same_inputs() {
        let order = PreparedExchangeOrder {
            surface: OrderSurface::Ticket,
            symbol_key: "BTC".to_string(),
            asset: 0,
            is_buy: true,
            price: "100".to_string(),
            size: "1".to_string(),
            order_kind: ExchangeOrderKind::Limit,
            reduce_only: false,
            market_type: MarketType::Perp,
        };

        let first = one_shot_place_cloid("0xabc", 1_000, &order);
        let same = one_shot_place_cloid("0xabc", 1_000, &order);
        let next = one_shot_place_cloid("0xabc", 1_001, &order);

        assert_eq!(first, same);
        assert_ne!(first, next);
        assert_eq!(first.len(), 34);
        assert!(first.starts_with("0x"));
        assert!(first[2..].chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    #[test]
    fn one_shot_placement_context_debug_redacts_account_address() {
        const ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";
        const CLOID: &str = "0xdeadbeef";
        const SYMBOL: &str = "SECRETCOIN";
        let context = OneShotPlacementContext {
            account_address: ACCOUNT.to_string(),
            cloid: CLOID.to_string(),
            surface: OrderSurface::Ticket,
            symbol_key: SYMBOL.to_string(),
            order_kind: ExchangeOrderKind::Limit,
        };

        let rendered = format!("{context:?}");

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains(ACCOUNT));
        assert!(!rendered.contains(CLOID));
        assert!(!rendered.contains(SYMBOL));
    }

    #[test]
    fn prepared_order_can_build_request_with_strategy_supplied_cloid() {
        let order = PreparedExchangeOrder {
            surface: OrderSurface::Chase,
            symbol_key: "BTC".to_string(),
            asset: 0,
            is_buy: true,
            price: "100".to_string(),
            size: "1".to_string(),
            order_kind: ExchangeOrderKind::Limit,
            reduce_only: false,
            market_type: MarketType::Perp,
        };

        let request = order
            .place_request_with_existing_cloid("0x11111111111111111111111111111111".to_string());

        assert_eq!(request.asset, 0);
        assert_eq!(request.price, "100");
        assert_eq!(request.size, "1");
        assert_eq!(
            request.cloid,
            Some("0x11111111111111111111111111111111".to_string())
        );
    }

    #[test]
    fn one_shot_cloid_nonce_allocator_is_monotonic() {
        let nonce = AtomicU64::new(0);

        let first = allocate_one_shot_cloid_nonce_from(&nonce, 10);
        let second = allocate_one_shot_cloid_nonce_from(&nonce, 10);
        let third = allocate_one_shot_cloid_nonce_from(&nonce, 9);

        assert_eq!(first, 10);
        assert_eq!(second, 11);
        assert_eq!(third, 12);
    }
}

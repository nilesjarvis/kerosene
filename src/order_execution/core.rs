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
use std::fmt::Write as _;
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlaceIntent {
    pub(crate) surface: OrderSurface,
    pub(crate) symbol_key: String,
    pub(crate) is_buy: bool,
    pub(crate) order_kind: ExchangeOrderKind,
    pub(crate) price_source: PriceSource,
    pub(crate) quantity_source: QuantitySource,
    pub(crate) reduce_only_source: ReduceOnlySource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CancelIntent {
    pub(crate) surface: OrderSurface,
    pub(crate) symbol_key: String,
    pub(crate) oid: u64,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarketUsdSizeReference {
    ExecutionPrice,
    Mid,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OneShotPlacementContext {
    pub(crate) account_address: String,
    pub(crate) cloid: String,
    pub(crate) surface: OrderSurface,
    pub(crate) symbol_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedCancelOrder {
    pub(crate) surface: OrderSurface,
    pub(crate) symbol_key: String,
    pub(crate) asset: u32,
    pub(crate) oid: u64,
    pub(crate) market_type: MarketType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
                Self::QuickOrder | Self::Hud | Self::ClosePosition | Self::Chase | Self::Twap => {
                    market_type != MarketType::Outcome
                }
                Self::Nuke => market_type == MarketType::Perp,
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
            (Self::Chase, OrderOperation::Place) => "chase trading",
            (Self::Twap, OrderOperation::Place) => "TWAP trading",
            _ => operation.label(),
        }
    }

    fn orderability_context_label(self) -> &'static str {
        match self {
            Self::Ticket | Self::Preset | Self::Chase | Self::Twap => "Active",
            Self::QuickOrder | Self::Hud => "Chart",
            Self::ClosePosition | Self::Nuke => "Position",
            Self::Move | Self::Cancel => "Order",
        }
    }

    fn symbol_not_found_status_text(self, symbol_key: &str) -> String {
        match self {
            Self::QuickOrder
            | Self::Hud
            | Self::ClosePosition
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
    pub(crate) fn prepare_cancel_order(
        &self,
        intent: CancelIntent,
    ) -> Result<PreparedCancelOrder, String> {
        let Some(sym) = self
            .exchange_symbols
            .iter()
            .find(|symbol| symbol.key == intent.symbol_key)
        else {
            return Err(intent
                .surface
                .symbol_not_found_status_text(&intent.symbol_key));
        };
        validate_surface_market_type(intent.surface, OrderOperation::Cancel, sym.market_type)
            .map_err(OrderCapabilityError::status_text)?;

        Ok(PreparedCancelOrder {
            surface: intent.surface,
            symbol_key: sym.key.clone(),
            asset: sym.asset_index,
            oid: intent.oid,
            market_type: sym.market_type,
        })
    }

    pub(crate) fn prepare_modify_order(
        &self,
        intent: ModifyIntent,
    ) -> Result<PreparedModifyOrderResult, String> {
        let Some(sym) = self
            .exchange_symbols
            .iter()
            .find(|symbol| symbol.key == intent.symbol_key)
        else {
            return Err(intent
                .surface
                .symbol_not_found_status_text(&intent.symbol_key));
        };
        self.validate_exchange_symbol_orderable(sym, intent.surface.orderability_context_label())?;
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
        let Some(sym) = self
            .exchange_symbols
            .iter()
            .find(|symbol| symbol.key == intent.symbol_key)
        else {
            return Err(intent
                .surface
                .symbol_not_found_status_text(&intent.symbol_key));
        };
        self.validate_exchange_symbol_orderable(sym, intent.surface.orderability_context_label())?;
        validate_surface_market_type(intent.surface, OrderOperation::Place, sym.market_type)
            .map_err(OrderCapabilityError::status_text)?;

        let symbol_key = sym.key.as_str();
        let sz_decimals = sym.sz_decimals;
        let is_outcome = sym.market_type == MarketType::Outcome;
        let is_spot_like = Self::market_type_is_spot_like(sym.market_type);

        let raw_qty = match &intent.quantity_source {
            QuantitySource::UserInput {
                value,
                invalid_message,
                ..
            } => parse_positive_number(value).ok_or_else(|| (*invalid_message).to_string()),
            QuantitySource::CoinSize {
                size,
                invalid_message,
                ..
            } => positive_finite_value(*size).ok_or_else(|| (*invalid_message).to_string()),
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
                        symbol_key,
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
                        symbol_key,
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

        let quantity_is_usd = match intent.quantity_source {
            QuantitySource::UserInput { denomination, .. } => {
                !is_outcome && denomination == QuantityDenomination::UsdNotional
            }
            QuantitySource::CoinSize { .. } => false,
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
        };
        let qty = order_size_from_quantity_input(
            raw_qty,
            usd_size_reference_price,
            quantity_is_usd,
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
            OrderSurface::Chase,
            OrderSurface::Twap,
        ] {
            assert!(!surface.allows_market_type(OrderOperation::Place, MarketType::Outcome));
        }
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
        terminal.exchange_symbols = vec![symbol("PURR/USDC", MarketType::Spot)];
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

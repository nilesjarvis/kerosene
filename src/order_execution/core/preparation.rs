use super::{
    CancelIntent, MarketUsdSizeReference, ModifyIntent, OrderCapabilityError, OrderOperation,
    OrderSurface, PlaceIntent, PreparedCancelOrder, PreparedExchangeOrder, PreparedModifyOrder,
    PreparedModifyOrderResult, PriceSource, QuantityDenomination, QuantitySource, ReduceOnlySource,
    validate_surface_market_type,
};
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::{finite_value, parse_positive_number, positive_finite_value};
use crate::order_execution::pricing::{rounded_market_price, slipped_market_price};
use crate::order_execution::sizing::order_size_from_quantity_input;
use crate::signing::{float_to_wire, round_price};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Order Preparation
// ---------------------------------------------------------------------------

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
        // unavailable. HIP-4 also has deterministic canonical side keys.
        // Keep this fallback cancellation-only: placement and
        // modification still require complete metadata for decimals,
        // orderability, and market-type validation.
        let recovered = metadata_free_spot_cancel_asset(&intent.symbol_key)
            .map(|asset| (asset, MarketType::Spot))
            .or_else(|| {
                metadata_free_outcome_cancel_asset(&intent.symbol_key)
                    .map(|asset| (asset, MarketType::Outcome))
            });
        let Some((asset, market_type)) = recovered else {
            return Err(intent
                .surface
                .symbol_not_found_status_text(&intent.symbol_key));
        };
        validate_surface_market_type(intent.surface, OrderOperation::Cancel, market_type)
            .map_err(OrderCapabilityError::status_text)?;

        Ok(PreparedCancelOrder {
            surface: intent.surface,
            symbol_key: intent.symbol_key,
            asset,
            oid: intent.oid,
            market_type,
        })
    }

    pub(crate) fn prepare_modify_order(
        &self,
        intent: ModifyIntent<'_>,
    ) -> Result<PreparedModifyOrderResult, String> {
        let Some(sym) = self.exchange_symbol_for_key(intent.symbol_key) else {
            return Err(intent
                .surface
                .symbol_not_found_status_text(intent.symbol_key));
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

/// Settled outcomes can disappear from metadata before their open-order
/// snapshot refreshes. Only canonical HIP-4 side keys can recover an asset,
/// and this helper is used exclusively for cancellation.
fn metadata_free_outcome_cancel_asset(key: &str) -> Option<u32> {
    let encoded = key.strip_prefix('#')?;
    if encoded.is_empty()
        || (encoded.len() > 1 && encoded.starts_with('0'))
        || !encoded.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let encoding = encoded.parse::<u32>().ok()?;
    if encoding % 10 > 1 {
        return None;
    }
    crate::api::OUTCOME_ASSET_ID_OFFSET.checked_add(encoding)
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

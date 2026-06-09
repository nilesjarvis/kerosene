use crate::app_state::TradingTerminal;
use crate::helpers::{finite_value, positive_finite_value};
use crate::message::Message;
use crate::order_execution::{
    MarketUsdSizeReference, OrderSurface, PendingOrderAction, PlaceIntent, PreparedExchangeOrder,
    PriceSource, QuantitySource, ReduceOnlySource, place_order_task,
};
use crate::signing::ExchangeOrderKind;

use iced::Task;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClosePositionInputError {
    InvalidPositionSize,
    InvalidFraction,
}

fn close_position_order_side_and_size(
    raw_szi: &str,
    fraction: f64,
) -> Result<(bool, f64), ClosePositionInputError> {
    let Some(fraction) = positive_finite_value(fraction) else {
        return Err(ClosePositionInputError::InvalidFraction);
    };
    if fraction > 1.0 {
        return Err(ClosePositionInputError::InvalidFraction);
    }

    let szi = raw_szi
        .trim()
        .parse::<f64>()
        .map_err(|_| ClosePositionInputError::InvalidPositionSize)?;
    let Some(szi) = finite_value(szi) else {
        return Err(ClosePositionInputError::InvalidPositionSize);
    };
    if szi.abs() <= 1e-12 {
        return Err(ClosePositionInputError::InvalidPositionSize);
    }

    let is_buy = szi < 0.0;
    let close_size = szi.abs() * fraction;
    Ok((is_buy, close_size))
}

impl TradingTerminal {
    pub(crate) fn execute_close_position(
        &mut self,
        coin: &str,
        fraction: f64,
        use_market: bool,
    ) -> Task<Message> {
        let _theme = self.theme();
        // The close menu closes after the first click, but a second queued
        // click still dispatches; without this gate a double-fired partial
        // close stacks (two 50% closes flatten the position).
        if self.pending_order_action.is_some() {
            self.order_status = Some(("Wait for the pending order action to finish".into(), true));
            return Task::none();
        }
        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() || self.connected_address.is_none() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        }
        if self.account_loading {
            self.order_status = Some((
                "Account refresh in progress; wait for fresh account data before closing".into(),
                true,
            ));
            return Task::none();
        }

        let Some(account_data) = self.account_data.as_ref() else {
            self.order_status = Some((
                "No account data available; refresh before closing".into(),
                true,
            ));
            return Task::none();
        };
        let now_ms = Self::now_ms();
        if !account_data.is_fresh_for_position_action(now_ms) {
            let age_label = account_data
                .position_action_snapshot_age_ms(now_ms)
                .map(|age| format!("{}s old", age.div_ceil(1000)))
                .unwrap_or_else(|| "from the future".to_string());
            self.order_status = Some((
                format!("Account data is stale ({age_label}); refresh before closing positions"),
                true,
            ));
            return self.refresh_account_data();
        }

        let pos = account_data
            .clearinghouse
            .asset_positions
            .iter()
            .find(|ap| ap.position.coin == coin)
            .map(|ap| &ap.position);
        let Some(pos) = pos else {
            self.order_status = Some((format!("No position found for {coin}"), true));
            return Task::none();
        };

        let (is_buy, size) = match close_position_order_side_and_size(&pos.szi, fraction) {
            Ok(inputs) => inputs,
            Err(ClosePositionInputError::InvalidPositionSize) => {
                self.order_status = Some(("Position size is invalid".into(), true));
                return Task::none();
            }
            Err(ClosePositionInputError::InvalidFraction) => {
                self.order_status = Some(("Close fraction is invalid".into(), true));
                return Task::none();
            }
        };

        let order_kind = if use_market {
            ExchangeOrderKind::Market
        } else {
            ExchangeOrderKind::Limit
        };
        let intent = PlaceIntent {
            surface: OrderSurface::ClosePosition,
            symbol_key: coin.to_string(),
            is_buy,
            order_kind,
            price_source: if use_market {
                PriceSource::MarketWithSlippage {
                    invalid_message: None,
                    usd_size_reference: MarketUsdSizeReference::ExecutionPrice,
                }
            } else {
                PriceSource::ReferenceMid
            },
            quantity_source: QuantitySource::CoinSize {
                size,
                invalid_message: "Position size is invalid",
                precision_invalid_message: "Position size is invalid",
            },
            reduce_only_source: ReduceOnlySource::Fixed(true),
        };
        let prepared = match self.prepare_place_order(intent) {
            Ok(prepared) => prepared,
            Err(message) => {
                self.order_status = Some((message, true));
                return Task::none();
            }
        };

        self.submit_prepared_close_position_order(key, coin, fraction, use_market, prepared)
    }

    fn submit_prepared_close_position_order(
        &mut self,
        key: String,
        coin: &str,
        fraction: f64,
        use_market: bool,
        prepared: PreparedExchangeOrder,
    ) -> Task<Message> {
        let pct_label = format!("{:.0}%", fraction * 100.0);
        let kind_label = if use_market { "market" } else { "limit" };
        self.order_status = Some((
            format!("Closing {pct_label} of {coin} ({kind_label})..."),
            false,
        ));
        self.pending_order_action = Some(PendingOrderAction::ClosePosition);

        let account_address = self.connected_address.clone().unwrap_or_default();
        let (request, context) = prepared.place_request_with_context(&account_address);
        place_order_task(key.into(), request, move |r| Message::ClosePositionResult {
            context,
            result: Box::new(r),
        })
    }
}

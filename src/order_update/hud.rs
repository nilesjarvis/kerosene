use crate::app_state::TradingTerminal;
use crate::helpers::positive_finite_value;
use crate::message::Message;
use crate::order_execution::{
    HudOrderRequest, HudOrderType, MarketUsdSizeReference, OneShotPlacementContext, OrderSurface,
    PendingOrderAction, PlaceIntent, PreparedExchangeOrder, PriceSource, QuantityDenomination,
    QuantitySource, ReduceOnlySource, place_order_task,
};
use crate::order_update::results::classify_execution_result;
use crate::signing::{ExchangeOrderKind, ExchangeResponse};
use crate::sound;

use iced::{Point, Size, Task};

// ---------------------------------------------------------------------------
// HUD Chart Order Submission
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn handle_submit_hud_order(&mut self, request: HudOrderRequest) -> Task<Message> {
        let Some(instance) = self.charts.get(&request.chart_id) else {
            self.order_status = Some(("HUD chart is no longer available".into(), true));
            return Task::none();
        };
        if instance.chart.surface_id() != request.surface_id {
            self.order_status = Some(("HUD order ignored: chart surface changed".into(), true));
            return Task::none();
        }
        if !instance.chart.hud_order_submission_enabled() {
            self.order_status = Some((
                "HUD trading is in safe mode; arm the chart first".into(),
                true,
            ));
            return Task::none();
        }

        // Chart clicks can queue faster than results return; serialize HUD
        // submissions on the same pending flag the one-shot results clear.
        if self.pending_order_action.is_some() {
            self.order_status = Some(("Wait for the pending order action to finish".into(), true));
            return Task::none();
        }

        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() || self.connected_address.is_none() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        }

        let chart_symbol = instance.symbol.clone();
        if chart_symbol.is_empty() {
            self.order_status = Some(("Select a chart symbol before HUD trading".into(), true));
            return Task::none();
        }

        let is_market_order = request.order_type == HudOrderType::Market;
        let order_kind = if is_market_order {
            ExchangeOrderKind::Market
        } else {
            ExchangeOrderKind::Limit
        };
        let intent = PlaceIntent {
            surface: OrderSurface::Hud,
            symbol_key: chart_symbol,
            is_buy: if is_market_order {
                request.market_side.is_buy()
            } else {
                false
            },
            order_kind,
            price_source: match request.order_type {
                HudOrderType::Limit => PriceSource::LimitInput {
                    value: request.price.to_string(),
                    invalid_message: "Invalid HUD limit price",
                },
                HudOrderType::Market => PriceSource::MarketWithSlippage {
                    invalid_message: Some("Invalid HUD market price"),
                    usd_size_reference: MarketUsdSizeReference::ExecutionPrice,
                },
            },
            quantity_source: QuantitySource::UserInput {
                value: request.quantity.clone(),
                denomination: QuantityDenomination::Coin,
                invalid_message: "Invalid HUD order size",
                precision_invalid_message: "Invalid HUD size for asset precision",
            },
            reduce_only_source: ReduceOnlySource::Form(self.order_reduce_only),
        };
        let mut prepared = match self.prepare_place_order(intent) {
            Ok(prepared) => prepared,
            Err(message) => {
                self.order_status = Some((message, true));
                return Task::none();
            }
        };
        if !is_market_order {
            prepared.is_buy = match self.hud_limit_order_is_buy(&prepared.symbol_key, request.price)
            {
                Some(is_buy) => is_buy,
                None => {
                    self.order_status =
                        Some(("No reference price for HUD limit side".into(), true));
                    return Task::none();
                }
            };
        }

        self.submit_prepared_hud_order(key, request, prepared, is_market_order)
    }

    fn submit_prepared_hud_order(
        &mut self,
        key: String,
        request: HudOrderRequest,
        prepared: PreparedExchangeOrder,
        is_market_order: bool,
    ) -> Task<Message> {
        let kind_label = match request.order_type {
            HudOrderType::Limit => "limit",
            HudOrderType::Market => "market",
        };
        let side_label = if prepared.is_buy { "LONG" } else { "SHORT" };
        self.order_status = Some((
            format!(
                "Placing HUD {kind_label} {side_label} {} {}...",
                prepared.size, prepared.symbol_key
            ),
            false,
        ));
        self.pending_order_action = Some(if prepared.is_buy {
            PendingOrderAction::Buy
        } else {
            PendingOrderAction::Sell
        });
        self.start_hud_order_animation(&request, prepared.is_buy, !is_market_order);
        if self.sound_enabled {
            sound::play_hud_order(
                self.chart_hud_order_sound,
                self.chart_hud_order_sound_path(),
                self.chart_hud_order_sound_volume,
            );
        }

        let account_address = self.connected_address.clone().unwrap_or_default();
        let pending_indicator_id = if is_market_order {
            self.add_pending_market_order_placement_indicator(
                account_address.clone(),
                prepared.symbol_key.clone(),
                prepared.is_buy,
                prepared.size.clone(),
                prepared.price.clone(),
            )
        } else {
            self.add_pending_order_placement_indicator(
                account_address.clone(),
                prepared.symbol_key.clone(),
                prepared.is_buy,
                prepared.size.clone(),
                prepared.price.clone(),
            )
        };

        let (request, context) = prepared.place_request_with_context(&account_address);
        place_order_task(key.into(), request, move |result| Message::HudOrderResult {
            pending_indicator_id,
            context,
            result: Box::new(result),
        })
    }

    pub(crate) fn handle_hud_order_result(
        &mut self,
        pending_indicator_id: Option<u64>,
        context: OneShotPlacementContext,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        self.pending_order_action = None;
        self.clear_pending_order_indicator(pending_indicator_id);
        let outcome = classify_execution_result(result);
        self.apply_one_shot_placement_outcome(context, outcome)
    }

    fn hud_limit_order_is_buy(&self, chart_symbol: &str, price: f64) -> Option<bool> {
        let reference = self
            .resolve_mid_for_symbol(chart_symbol)
            .or_else(|| {
                self.charts
                    .values()
                    .find(|inst| inst.symbol == chart_symbol)
                    .and_then(|inst| inst.chart.candles.last())
                    .map(|candle| candle.close)
            })
            .and_then(positive_finite_value)?;

        Some(price <= reference)
    }

    fn start_hud_order_animation(
        &mut self,
        request: &HudOrderRequest,
        is_buy: bool,
        show_line: bool,
    ) {
        let Some(instance) = self.charts.get_mut(&request.chart_id) else {
            return;
        };
        if instance.chart.surface_id() != request.surface_id {
            return;
        }

        instance.chart.start_hud_order_animation(
            request.price,
            Point::new(request.click_x, request.click_y),
            Size::new(request.chart_w, request.chart_h),
            is_buy,
            show_line,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ExchangeSymbol, MarketType};
    use crate::app_state::sensitive_string;
    use crate::chart_state::{ChartInstance, ChartSurfaceId};
    use crate::config::ChartCrosshairStyle;
    use crate::order_execution::{HudOrderSide, PendingOrderAction};
    use crate::timeframe::Timeframe;

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

    fn terminal_with_hud_chart(armed: bool) -> TradingTerminal {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.pending_order_action = None;
        terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
        terminal.wallet_key_input = sensitive_string("agent-key");

        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance.chart.set_crosshair_style(ChartCrosshairStyle::Hud);
        if armed {
            instance.chart.set_hud_armed_at(true, 1_000);
        }
        terminal.charts.insert(1, instance);
        terminal
    }

    fn hud_request(surface_id: ChartSurfaceId) -> HudOrderRequest {
        HudOrderRequest {
            chart_id: 1,
            surface_id,
            price: 100.0,
            quantity: "1".to_string(),
            order_type: HudOrderType::Limit,
            market_side: HudOrderSide::Long,
            click_x: 120.0,
            click_y: 80.0,
            chart_w: 400.0,
            chart_h: 240.0,
        }
    }

    #[test]
    fn hud_order_submission_rejects_safe_mode() {
        let mut terminal = terminal_with_hud_chart(false);

        let _task = terminal.handle_submit_hud_order(hud_request(ChartSurfaceId::Docked(1)));

        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("HUD trading is in safe mode; arm the chart first", true))
        );
        assert!(terminal.pending_order_action.is_none());
    }

    #[test]
    fn hud_order_submission_rejects_stale_surface() {
        let mut terminal = terminal_with_hud_chart(true);

        let _task = terminal.handle_submit_hud_order(hud_request(ChartSurfaceId::Docked(99)));

        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("HUD order ignored: chart surface changed", true))
        );
        assert!(terminal.pending_order_action.is_none());
    }

    #[test]
    fn hud_order_submission_rejects_while_order_action_pending() {
        let mut terminal = terminal_with_hud_chart(true);
        terminal.pending_order_action = Some(PendingOrderAction::Sell);

        let _task = terminal.handle_submit_hud_order(hud_request(ChartSurfaceId::Docked(1)));

        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("Wait for the pending order action to finish", true))
        );
        assert_eq!(
            terminal.pending_order_action,
            Some(PendingOrderAction::Sell)
        );
    }

    #[test]
    fn hud_order_result_clears_pending_order_action() {
        let mut terminal = terminal_with_hud_chart(true);
        terminal.pending_order_action = Some(PendingOrderAction::Buy);

        let _task = terminal.handle_hud_order_result(
            None,
            OneShotPlacementContext {
                account_address: "0xabc".to_string(),
                cloid: "0x00000000000000000000000000000000".to_string(),
                surface: OrderSurface::Hud,
                symbol_key: "BTC".to_string(),
            },
            Err("exchange request failed".into()),
        );

        assert!(terminal.pending_order_action.is_none());
    }

    #[test]
    fn hud_order_submission_uses_shared_preflight_quantity_error() {
        let mut terminal = terminal_with_hud_chart(true);
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        let mut request = hud_request(ChartSurfaceId::Docked(1));
        request.quantity = "0".to_string();

        let _task = terminal.handle_submit_hud_order(request);

        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("Invalid HUD order size", true))
        );
        assert!(terminal.pending_order_action.is_none());
    }
}

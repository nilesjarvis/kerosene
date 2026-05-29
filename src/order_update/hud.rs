use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::{parse_number, positive_finite_value};
use crate::message::Message;
use crate::order_execution::{
    HudOrderRequest, HudOrderType, PendingOrderAction, order_size_from_quantity_input,
    pricing::rounded_market_price,
};
use crate::order_update::results::result_requires_account_refresh;
use crate::signing::{ExchangeResponse, OrderKind, float_to_wire, place_order, round_price};
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
        if self.symbol_key_is_hidden(&chart_symbol) {
            self.order_status = Some(("Chart ticker is hidden in Settings > Risk".into(), true));
            return Task::none();
        }

        let Some(sym) = self
            .exchange_symbols
            .iter()
            .find(|symbol| symbol.key == chart_symbol)
        else {
            self.order_status = Some((format!("Symbol '{}' not found", chart_symbol), true));
            return Task::none();
        };
        if let Err(message) = self.validate_exchange_symbol_orderable(sym, "Chart") {
            self.order_status = Some((message, true));
            return Task::none();
        }
        if sym.market_type == MarketType::Outcome {
            self.outcome_read_only_status("HUD trading");
            return Task::none();
        }

        let asset = sym.asset_index;
        let sz_decimals = sym.sz_decimals;
        let market_type = sym.market_type;
        let is_spot_like = Self::market_type_is_spot_like(market_type);

        let raw_quantity = match parse_number(&request.quantity).and_then(positive_finite_value) {
            Some(quantity) => quantity,
            None => {
                self.order_status = Some(("Invalid HUD order size".into(), true));
                return Task::none();
            }
        };

        let is_market_order = request.order_type == HudOrderType::Market;
        let (is_buy, order_kind, price, reference_price) = match request.order_type {
            HudOrderType::Limit => {
                let Some(rounded) =
                    positive_finite_value(round_price(request.price, sz_decimals, is_spot_like))
                else {
                    self.order_status = Some(("Invalid HUD limit price".into(), true));
                    return Task::none();
                };
                if let Err(e) = self.validate_order_price_band(&chart_symbol, rounded) {
                    self.order_status = Some((e, true));
                    return Task::none();
                }
                let is_buy = match self.hud_limit_order_is_buy(&chart_symbol, request.price) {
                    Some(is_buy) => is_buy,
                    None => {
                        self.order_status =
                            Some(("No reference price for HUD limit side".into(), true));
                        return Task::none();
                    }
                };
                (is_buy, OrderKind::Limit, float_to_wire(rounded), rounded)
            }
            HudOrderType::Market => {
                let Some(mid) = self.resolve_mid_for_symbol(&chart_symbol) else {
                    self.order_status = Some((
                        format!(
                            "No mid price for {} (tried {})",
                            chart_symbol,
                            self.mid_candidates_for_symbol(&chart_symbol).join(", ")
                        ),
                        true,
                    ));
                    return Task::none();
                };
                let is_buy = request.market_side.is_buy();
                let Some(rounded) = positive_finite_value(rounded_market_price(
                    mid,
                    is_buy,
                    self.market_slippage_fraction(),
                    sz_decimals,
                    is_spot_like,
                )) else {
                    self.order_status = Some(("Invalid HUD market price".into(), true));
                    return Task::none();
                };
                if let Err(e) = self.validate_order_price_band(&chart_symbol, rounded) {
                    self.order_status = Some((e, true));
                    return Task::none();
                }
                (is_buy, OrderKind::Market, float_to_wire(rounded), mid)
            }
        };

        let Some(size) =
            order_size_from_quantity_input(raw_quantity, reference_price, false, sz_decimals)
                .map(float_to_wire)
        else {
            self.order_status = Some(("Invalid HUD size for asset precision".into(), true));
            return Task::none();
        };

        let reduce_only = if is_spot_like {
            false
        } else {
            self.order_reduce_only
        };
        let kind_label = match request.order_type {
            HudOrderType::Limit => "limit",
            HudOrderType::Market => "market",
        };
        let side_label = if is_buy { "LONG" } else { "SHORT" };
        self.order_status = Some((
            format!("Placing HUD {kind_label} {side_label} {size} {chart_symbol}..."),
            false,
        ));
        self.pending_order_action = Some(if is_buy {
            PendingOrderAction::Buy
        } else {
            PendingOrderAction::Sell
        });
        self.start_hud_order_animation(&request, is_buy, !is_market_order);
        if self.sound_enabled {
            sound::play_hud_order(
                self.chart_hud_order_sound,
                self.chart_hud_order_sound_path(),
                self.chart_hud_order_sound_volume,
            );
        }

        let pending_indicator_id = if is_market_order {
            self.add_pending_market_order_placement_indicator(
                self.connected_address.clone().unwrap_or_default(),
                chart_symbol,
                is_buy,
                size.clone(),
                price.clone(),
            )
        } else {
            self.add_pending_order_placement_indicator(
                self.connected_address.clone().unwrap_or_default(),
                chart_symbol,
                is_buy,
                size.clone(),
                price.clone(),
            )
        };

        Task::perform(
            place_order(
                key.into(),
                asset,
                is_buy,
                price,
                size,
                order_kind,
                reduce_only,
            ),
            move |result| Message::HudOrderResult {
                pending_indicator_id,
                result: Box::new(result),
            },
        )
    }

    pub(crate) fn handle_hud_order_result(
        &mut self,
        pending_indicator_id: Option<u64>,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        self.clear_pending_order_indicator(pending_indicator_id);
        let should_refresh = result_requires_account_refresh(&result);
        match result {
            Ok(resp) => {
                let is_err = resp.is_error();
                self.set_order_status(resp.summary(), is_err);
            }
            Err(e) => {
                self.set_order_status(e, true);
            }
        }
        if should_refresh {
            self.refresh_account_data()
        } else {
            Task::none()
        }
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
    use crate::app_state::sensitive_string;
    use crate::chart_state::{ChartInstance, ChartSurfaceId};
    use crate::config::ChartCrosshairStyle;
    use crate::order_execution::HudOrderSide;
    use crate::timeframe::Timeframe;

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
}

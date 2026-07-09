use crate::api::MarketType;
use crate::app_state::TradingTerminal;
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
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// HUD Chart Order Submission
// ---------------------------------------------------------------------------

/// Spot orders trade tokens the account holds, so perp LONG/SHORT wording
/// misrepresents them; label spot sides BUY/SELL instead.
fn hud_side_label(market_type: MarketType, is_buy: bool) -> &'static str {
    match (market_type == MarketType::Spot, is_buy) {
        (true, true) => "BUY",
        (true, false) => "SELL",
        (false, true) => "LONG",
        (false, false) => "SHORT",
    }
}

impl TradingTerminal {
    pub(crate) fn handle_submit_hud_order(&mut self, request: HudOrderRequest) -> Task<Message> {
        let Some(instance) = self.charts.get(&request.chart_id) else {
            self.set_order_status("HUD chart is no longer available".into(), true);
            return Task::none();
        };
        if instance.chart.surface_id() != request.surface_id {
            self.set_order_status("HUD order ignored: chart surface changed".into(), true);
            return Task::none();
        }
        let chart_symbol = instance.symbol.clone();
        if chart_symbol != request.symbol_key {
            self.set_order_status("HUD order ignored: chart symbol changed".into(), true);
            return Task::none();
        }
        if !instance.chart.hud_order_submission_enabled() {
            self.set_order_status(
                "HUD trading is in safe mode; arm the chart first".into(),
                true,
            );
            return Task::none();
        }

        if chart_symbol.is_empty() {
            self.set_order_status("Select a chart symbol before HUD trading".into(), true);
            return Task::none();
        }

        let is_market_order = request.order_type == HudOrderType::Market;
        let order_kind = if is_market_order {
            ExchangeOrderKind::Market
        } else {
            ExchangeOrderKind::Limit
        };
        let is_buy = if is_market_order {
            request.market_side.is_buy()
        } else {
            match request.limit_side {
                Some(side) => side.is_buy(),
                None => {
                    self.set_order_status("No click-time side for HUD limit order".into(), true);
                    return Task::none();
                }
            }
        };

        // HUD limit clicks may overlap each other (rapid click-trading), so
        // they only gate on non-HUD trading requests plus an in-flight cap.
        // Market clicks move position immediately at whatever the book gives,
        // so they keep the fully-serialized path: one at a time, and only on
        // fresh account data.
        if is_market_order {
            if self.reject_if_pending_trading_request("placing a HUD order") {
                self.toast_order_status();
                return Task::none();
            }
            if self.reject_if_account_reconciliation_required("placing a HUD order", "account data")
            {
                self.toast_order_status();
                return Task::none();
            }
        } else {
            if self.reject_if_pending_trading_request_blocking_hud_placement("placing a HUD order")
            {
                self.toast_order_status();
                return Task::none();
            }
            if self.reject_if_hud_placement_limit_reached() {
                self.toast_order_status();
                return Task::none();
            }
        }

        let Some((key, account_address)) = self.order_signing_context() else {
            self.toast_order_status();
            return Task::none();
        };
        let intent = PlaceIntent {
            surface: OrderSurface::Hud,
            symbol_key: chart_symbol,
            is_buy,
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
        let prepared = match self.prepare_place_order(intent) {
            Ok(prepared) => prepared,
            Err(message) => {
                self.set_order_status(message, true);
                return Task::none();
            }
        };

        self.submit_prepared_hud_order(key, account_address, request, prepared, is_market_order)
    }

    fn submit_prepared_hud_order(
        &mut self,
        key: Zeroizing<String>,
        account_address: String,
        request: HudOrderRequest,
        prepared: PreparedExchangeOrder,
        is_market_order: bool,
    ) -> Task<Message> {
        let kind_label = match request.order_type {
            HudOrderType::Limit => "limit",
            HudOrderType::Market => "market",
        };
        let side_label = hud_side_label(prepared.market_type, prepared.is_buy);
        // Spot symbol keys are raw "@{index}" pair indices; show the pair name.
        let display_symbol = self.display_name_for_symbol(&prepared.symbol_key);
        self.order_status = Some((
            format!(
                "Placing HUD {kind_label} {side_label} {} {display_symbol}...",
                prepared.size
            ),
            false,
        ));
        // Only the serialized market path owns the global pending flag;
        // concurrent limit placements are tracked per order below.
        if is_market_order {
            self.pending_order_action = Some(if prepared.is_buy {
                PendingOrderAction::Buy
            } else {
                PendingOrderAction::Sell
            });
        }
        self.start_hud_order_animation(&request, prepared.is_buy, !is_market_order);
        self.push_hud_feed_entry(&request, &prepared);
        if self.sound_enabled {
            sound::play_hud_order(
                self.chart_hud_order_sound,
                self.chart_hud_order_sound_path(),
                self.chart_hud_order_sound_volume,
            );
        }

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

        let inflight_id = (!is_market_order).then(|| {
            self.hud_placements.begin(
                account_address.clone(),
                pending_indicator_id,
                Self::now_ms(),
            )
        });

        let (request, context) = prepared.place_request_with_context(&account_address);
        self.invalidate_spot_balances_after_exchange_dispatch(
            &account_address,
            prepared.market_type,
        );
        place_order_task(key, request, move |result| Message::HudOrderResult {
            pending_indicator_id,
            inflight_id,
            context,
            result: Box::new(result),
        })
    }

    pub(crate) fn handle_hud_order_result(
        &mut self,
        pending_indicator_id: Option<u64>,
        inflight_id: Option<u64>,
        context: OneShotPlacementContext,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        if let Some(inflight_id) = inflight_id {
            self.hud_placements.finish(inflight_id);
        }
        // Only serialized market placements set the global flag; a limit
        // result must not clobber another surface's in-flight state.
        if context.order_kind == ExchangeOrderKind::Market {
            self.pending_order_action = None;
        }
        self.clear_pending_order_indicator(pending_indicator_id);
        let outcome = classify_execution_result(result);
        self.apply_one_shot_placement_outcome(context, outcome)
    }

    fn push_hud_feed_entry(&mut self, request: &HudOrderRequest, prepared: &PreparedExchangeOrder) {
        let Some(instance) = self.charts.get_mut(&request.chart_id) else {
            return;
        };
        if instance.chart.surface_id() != request.surface_id {
            return;
        }

        let kind_label = match request.order_type {
            HudOrderType::Limit => "LIMIT",
            HudOrderType::Market => "MKT",
        };
        let side_label = hud_side_label(prepared.market_type, prepared.is_buy);
        let label = format!(
            "{kind_label} {side_label} {} @ {}",
            prepared.size, prepared.price
        );
        instance
            .chart
            .push_hud_feed(label, prepared.is_buy, Self::now_ms());
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
    use crate::config::{AccountProfile, ChartCrosshairStyle};
    use crate::order_execution::{HudOrderSide, MAX_INFLIGHT_HUD_PLACEMENTS, PendingOrderAction};
    use crate::order_update::PendingOneShotStatusRequest;
    use crate::timeframe::Timeframe;

    const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

    fn connect_test_account(terminal: &mut TradingTerminal) {
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal.accounts = vec![AccountProfile {
            secret_id: "acct-a".to_string(),
            name: "Account A".to_string(),
            wallet_address: TEST_ACCOUNT.to_string(),
            agent_key: sensitive_string("").into_zeroizing(),
            hydromancer_api_key: sensitive_string("").into_zeroizing(),
        }];
        terminal.active_account_index = 0;
    }

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
        connect_test_account(&mut terminal);
        terminal.set_committed_agent_key_for_test("agent-key");

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
            symbol_key: "BTC".to_string(),
            price: 100.0,
            quantity: "1".to_string(),
            order_type: HudOrderType::Limit,
            market_side: HudOrderSide::Long,
            limit_side: Some(HudOrderSide::Long),
            click_x: 120.0,
            click_y: 80.0,
            chart_w: 400.0,
            chart_h: 240.0,
        }
    }

    fn pending_one_shot_status_request(surface: OrderSurface) -> PendingOneShotStatusRequest {
        PendingOneShotStatusRequest::new(
            7,
            &OneShotPlacementContext {
                account_address: TEST_ACCOUNT.to_string(),
                cloid: "0x00000000000000000000000000000003".to_string(),
                surface,
                symbol_key: "BTC".to_string(),
                order_kind: ExchangeOrderKind::Limit,
            },
        )
    }

    fn make_btc_tradeable(terminal: &mut TradingTerminal) {
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.all_mids.insert("BTC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
    }

    fn order_status_of(terminal: &TradingTerminal) -> Option<(&str, bool)> {
        terminal
            .order_status
            .as_ref()
            .map(|(message, is_error)| (message.as_str(), *is_error))
    }

    fn error_toast_messages(terminal: &TradingTerminal) -> Vec<&str> {
        terminal
            .toasts
            .iter()
            .filter(|toast| toast.is_error)
            .map(|toast| toast.message.as_str())
            .collect()
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
    fn hud_order_submission_safe_mode_rejection_pushes_toast() {
        // HUD trading happens on charts, where the order ticket pane may be
        // closed; rejections must surface as a toast, not only in the
        // pane-local status line.
        let mut terminal = terminal_with_hud_chart(false);

        let _task = terminal.handle_submit_hud_order(hud_request(ChartSurfaceId::Docked(1)));

        assert_eq!(
            error_toast_messages(&terminal),
            vec!["HUD trading is in safe mode; arm the chart first"]
        );
    }

    #[test]
    fn hud_order_submission_pending_gate_rejection_pushes_toast() {
        let mut terminal = terminal_with_hud_chart(true);
        terminal.pending_order_action = Some(PendingOrderAction::Sell);

        let _task = terminal.handle_submit_hud_order(hud_request(ChartSurfaceId::Docked(1)));

        assert_eq!(
            error_toast_messages(&terminal),
            vec!["Wait for pending trading requests to finish before placing a HUD order"]
        );
    }

    #[test]
    fn hud_order_submission_preflight_failure_pushes_toast() {
        let mut terminal = terminal_with_hud_chart(true);
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        let mut request = hud_request(ChartSurfaceId::Docked(1));
        request.quantity = "0".to_string();

        let _task = terminal.handle_submit_hud_order(request);

        assert_eq!(
            error_toast_messages(&terminal),
            vec!["Invalid HUD order size"]
        );
    }

    #[test]
    fn hud_spot_limit_submission_labels_sell_and_pair_name() {
        // Spot has no shorting and the WS key is a raw "@{index}" pair index;
        // the HUD status must read BUY/SELL with the pair name.
        let mut terminal = terminal_with_hud_chart(true);
        let mut spot = symbol("@107", MarketType::Spot);
        spot.display_name = Some("HYPE/USDC".to_string());
        terminal.exchange_symbols = vec![spot];
        terminal.all_mids.insert("@107".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("@107".to_string(), TradingTerminal::now_ms());
        terminal
            .charts
            .get_mut(&1)
            .expect("chart should exist")
            .symbol = "@107".to_string();
        let mut request = hud_request(ChartSurfaceId::Docked(1));
        request.symbol_key = "@107".to_string();
        request.price = 110.0;
        request.limit_side = Some(HudOrderSide::Short);

        let _task = terminal.handle_submit_hud_order(request);

        // Limit placements track in-flight state per order instead of the
        // global pending flag, so overlapping clicks stay possible.
        assert!(terminal.pending_order_action.is_none());
        assert_eq!(terminal.hud_placements.count_for_account(TEST_ACCOUNT), 1);
        assert_eq!(
            order_status_of(&terminal),
            Some(("Placing HUD limit SELL 1 HYPE/USDC...", false))
        );
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
    fn hud_order_submission_rejects_symbol_mismatch_after_click() {
        let mut terminal = terminal_with_hud_chart(true);
        terminal.exchange_symbols = vec![
            symbol("BTC", MarketType::Perp),
            symbol("ETH", MarketType::Perp),
        ];
        let mut request = hud_request(ChartSurfaceId::Docked(1));
        request.symbol_key = "BTC".to_string();
        terminal
            .charts
            .get_mut(&1)
            .expect("chart should exist")
            .symbol = "ETH".to_string();

        let _task = terminal.handle_submit_hud_order(request);

        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("HUD order ignored: chart symbol changed", true))
        );
        assert!(terminal.pending_order_action.is_none());
        assert!(terminal.pending_order_indicators.is_empty());
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
            Some((
                "Wait for pending trading requests to finish before placing a HUD order",
                true
            ))
        );
        assert_eq!(
            terminal.pending_order_action,
            Some(PendingOrderAction::Sell)
        );
    }

    #[test]
    fn hud_limit_submission_rejects_while_non_hud_one_shot_status_pending() {
        let mut terminal = terminal_with_hud_chart(true);
        terminal.insert_pending_one_shot_status_request(pending_one_shot_status_request(
            OrderSurface::Ticket,
        ));

        let _task = terminal.handle_submit_hud_order(hud_request(ChartSurfaceId::Docked(1)));

        assert_eq!(
            order_status_of(&terminal),
            Some((
                "Wait for pending trading requests to finish before placing a HUD order",
                true
            ))
        );
        assert!(terminal.pending_order_action.is_none());
        assert!(terminal.has_pending_one_shot_status_requests_for_test());
        assert!(terminal.pending_order_indicators.is_empty());
    }

    #[test]
    fn hud_limit_submission_allowed_while_hud_one_shot_status_pending() {
        // An earlier HUD click whose ack came back ambiguous resolves via its
        // own one-shot status check; that check must not block further limit
        // clicks (each cloid resolves independently).
        let mut terminal = terminal_with_hud_chart(true);
        make_btc_tradeable(&mut terminal);
        terminal.insert_pending_one_shot_status_request(pending_one_shot_status_request(
            OrderSurface::Hud,
        ));

        let _task = terminal.handle_submit_hud_order(hud_request(ChartSurfaceId::Docked(1)));

        assert_eq!(
            order_status_of(&terminal),
            Some(("Placing HUD limit LONG 1 BTC...", false))
        );
        assert_eq!(terminal.pending_order_indicators.len(), 1);
        assert_eq!(terminal.hud_placements.count_for_account(TEST_ACCOUNT), 1);
    }

    #[test]
    fn hud_market_submission_rejects_while_account_reconciliation_is_pending() {
        let mut terminal = terminal_with_hud_chart(true);
        terminal.account_reconciliation_required = true;
        let mut request = hud_request(ChartSurfaceId::Docked(1));
        request.order_type = HudOrderType::Market;

        let _task = terminal.handle_submit_hud_order(request);

        assert_eq!(
            order_status_of(&terminal),
            Some((
                "Account refresh pending; wait for fresh account data before placing a HUD order",
                true
            ))
        );
        assert!(terminal.pending_order_action.is_none());
        assert!(terminal.pending_order_indicators.is_empty());
    }

    #[test]
    fn hud_limit_submission_allowed_while_account_reconciliation_is_pending() {
        // Rapid limit clicking would otherwise stall on the account refresh
        // that follows every ack; the exchange stays authoritative for
        // margin, so limits skip the reconciliation gate.
        let mut terminal = terminal_with_hud_chart(true);
        make_btc_tradeable(&mut terminal);
        terminal.account_reconciliation_required = true;

        let _task = terminal.handle_submit_hud_order(hud_request(ChartSurfaceId::Docked(1)));

        assert_eq!(
            order_status_of(&terminal),
            Some(("Placing HUD limit LONG 1 BTC...", false))
        );
        assert_eq!(terminal.pending_order_indicators.len(), 1);
    }

    #[test]
    fn hud_market_order_result_clears_pending_order_action() {
        let mut terminal = terminal_with_hud_chart(true);
        terminal.pending_order_action = Some(PendingOrderAction::Buy);

        let _task = terminal.handle_hud_order_result(
            None,
            None,
            OneShotPlacementContext {
                account_address: "0xabc".to_string(),
                cloid: "0x00000000000000000000000000000000".to_string(),
                surface: OrderSurface::Hud,
                symbol_key: "BTC".to_string(),
                order_kind: ExchangeOrderKind::Market,
            },
            Err("exchange request failed".into()),
        );

        assert!(terminal.pending_order_action.is_none());
    }

    #[test]
    fn hud_limit_order_result_releases_inflight_slot_and_keeps_pending_action() {
        let mut terminal = terminal_with_hud_chart(true);
        // Limit placements never own the global flag, so a returning limit
        // result must not clobber another surface's in-flight state.
        terminal.pending_order_action = Some(PendingOrderAction::Sell);
        let inflight_id = terminal
            .hud_placements
            .begin(TEST_ACCOUNT.to_string(), None, 1_000);

        let _task = terminal.handle_hud_order_result(
            None,
            Some(inflight_id),
            OneShotPlacementContext {
                account_address: TEST_ACCOUNT.to_string(),
                cloid: "0x00000000000000000000000000000000".to_string(),
                surface: OrderSurface::Hud,
                symbol_key: "BTC".to_string(),
                order_kind: ExchangeOrderKind::Limit,
            },
            Err("exchange request failed".into()),
        );

        assert_eq!(
            terminal.pending_order_action,
            Some(PendingOrderAction::Sell)
        );
        assert!(!terminal.hud_placements.has_any_for_account(TEST_ACCOUNT));
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

    #[test]
    fn hud_limit_submission_uses_click_time_side_when_mid_moves() {
        let mut terminal = terminal_with_hud_chart(true);
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.all_mids.insert("BTC".to_string(), 90.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
        terminal.order_reduce_only = true;
        let mut request = hud_request(ChartSurfaceId::Docked(1));
        request.price = 100.0;
        request.limit_side = Some(HudOrderSide::Long);

        let _task = terminal.handle_submit_hud_order(request);

        assert!(terminal.pending_order_action.is_none());
        assert_eq!(terminal.hud_placements.count_for_account(TEST_ACCOUNT), 1);
        assert_eq!(
            order_status_of(&terminal),
            Some(("Placing HUD limit LONG 1 BTC...", false))
        );
        let indicator = terminal
            .pending_order_indicators
            .values()
            .next()
            .expect("HUD pending indicator");
        assert_eq!(indicator.account_address, TEST_ACCOUNT);
    }

    #[test]
    fn hud_limit_submission_uses_click_time_short_side() {
        let mut terminal = terminal_with_hud_chart(true);
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.all_mids.insert("BTC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
        let mut request = hud_request(ChartSurfaceId::Docked(1));
        request.price = 110.0;
        request.limit_side = Some(HudOrderSide::Short);

        let _task = terminal.handle_submit_hud_order(request);

        assert!(terminal.pending_order_action.is_none());
        assert_eq!(
            order_status_of(&terminal),
            Some(("Placing HUD limit SHORT 1 BTC...", false))
        );
    }

    #[test]
    fn hud_limit_submission_rejects_missing_click_time_side() {
        let mut terminal = terminal_with_hud_chart(true);
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.all_mids.insert("BTC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
        let mut request = hud_request(ChartSurfaceId::Docked(1));
        request.limit_side = None;

        let _task = terminal.handle_submit_hud_order(request);

        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("No click-time side for HUD limit order", true))
        );
        assert!(terminal.pending_order_action.is_none());
    }

    #[test]
    fn hud_limit_submission_rejects_missing_click_time_side_before_quantity_preflight() {
        let mut terminal = terminal_with_hud_chart(true);
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.all_mids.insert("BTC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
        let mut request = hud_request(ChartSurfaceId::Docked(1));
        request.limit_side = None;
        request.quantity = "0".to_string();

        let _task = terminal.handle_submit_hud_order(request);

        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("No click-time side for HUD limit order", true))
        );
        assert!(terminal.pending_order_action.is_none());
        assert!(terminal.pending_order_indicators.is_empty());
    }

    #[test]
    fn hud_limit_submission_rejects_missing_click_time_side_before_signing_context() {
        let mut terminal = terminal_with_hud_chart(true);
        terminal.accounts.clear();
        terminal.set_committed_agent_key_for_test("");
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.all_mids.insert("BTC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
        let mut request = hud_request(ChartSurfaceId::Docked(1));
        request.limit_side = None;

        let _task = terminal.handle_submit_hud_order(request);

        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("No click-time side for HUD limit order", true))
        );
        assert!(terminal.pending_order_action.is_none());
        assert!(terminal.pending_order_indicators.is_empty());
    }

    // ---- Concurrent limit placement ----

    #[test]
    fn hud_limit_submissions_can_overlap() {
        let mut terminal = terminal_with_hud_chart(true);
        make_btc_tradeable(&mut terminal);

        let _task = terminal.handle_submit_hud_order(hud_request(ChartSurfaceId::Docked(1)));
        let _task = terminal.handle_submit_hud_order(hud_request(ChartSurfaceId::Docked(1)));

        assert_eq!(terminal.hud_placements.count_for_account(TEST_ACCOUNT), 2);
        assert_eq!(terminal.pending_order_indicators.len(), 2);
        assert_eq!(error_toast_messages(&terminal), Vec::<&str>::new());
        assert_eq!(
            order_status_of(&terminal),
            Some(("Placing HUD limit LONG 1 BTC...", false))
        );
    }

    #[test]
    fn hud_limit_submission_rejects_at_inflight_cap() {
        let mut terminal = terminal_with_hud_chart(true);
        make_btc_tradeable(&mut terminal);
        for _ in 0..MAX_INFLIGHT_HUD_PLACEMENTS {
            terminal
                .hud_placements
                .begin(TEST_ACCOUNT.to_string(), None, 1_000);
        }

        let _task = terminal.handle_submit_hud_order(hud_request(ChartSurfaceId::Docked(1)));

        assert_eq!(
            order_status_of(&terminal),
            Some((
                "Too many HUD orders in flight; wait for confirmations",
                true
            ))
        );
        assert_eq!(
            error_toast_messages(&terminal),
            vec!["Too many HUD orders in flight; wait for confirmations"]
        );
        assert!(terminal.pending_order_indicators.is_empty());
        assert_eq!(
            terminal.hud_placements.count_for_account(TEST_ACCOUNT),
            MAX_INFLIGHT_HUD_PLACEMENTS
        );
    }

    #[test]
    fn hud_market_submission_rejects_while_hud_limit_inflight() {
        // Market clicks keep the fully-serialized path: an in-flight limit
        // burst must finish before a market order fires.
        let mut terminal = terminal_with_hud_chart(true);
        make_btc_tradeable(&mut terminal);
        terminal
            .hud_placements
            .begin(TEST_ACCOUNT.to_string(), None, 1_000);
        let mut request = hud_request(ChartSurfaceId::Docked(1));
        request.order_type = HudOrderType::Market;

        let _task = terminal.handle_submit_hud_order(request);

        assert_eq!(
            order_status_of(&terminal),
            Some((
                "Wait for pending trading requests to finish before placing a HUD order",
                true
            ))
        );
        assert!(terminal.pending_order_action.is_none());
        assert!(terminal.pending_order_indicators.is_empty());
    }

    #[test]
    fn inflight_hud_placement_blocks_other_trading_surfaces() {
        let mut terminal = terminal_with_hud_chart(true);
        assert!(!terminal.has_pending_trading_request());

        terminal
            .hud_placements
            .begin(TEST_ACCOUNT.to_string(), None, 1_000);

        assert!(terminal.has_pending_trading_request());
    }

    #[test]
    fn hud_limit_submission_rejects_while_non_hud_indicator_pending() {
        // A placement indicator the HUD tracker does not own (e.g. a ticket
        // order still awaiting its ack) keeps gating HUD clicks.
        let mut terminal = terminal_with_hud_chart(true);
        make_btc_tradeable(&mut terminal);
        terminal.add_pending_order_placement_indicator(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            true,
            "1".to_string(),
            "100".to_string(),
        );

        let _task = terminal.handle_submit_hud_order(hud_request(ChartSurfaceId::Docked(1)));

        assert_eq!(
            order_status_of(&terminal),
            Some((
                "Wait for pending trading requests to finish before placing a HUD order",
                true
            ))
        );
        assert!(terminal.hud_placements.count_for_account(TEST_ACCOUNT) == 0);
    }
}

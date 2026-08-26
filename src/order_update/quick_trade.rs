use crate::app_state::TradingTerminal;
use crate::config::QuickTradeDenomination;
use crate::message::Message;
use crate::order_execution::{
    MarketUsdSizeReference, OneShotPlacementContext, OrderSurface, PendingOrderAction, PlaceIntent,
    PriceSource, QuantityDenomination, QuantitySource, QuickTradeOrderRequest, ReduceOnlySource,
    place_order_task,
};
use crate::order_update::results::classify_execution_result;
use crate::signing::{ExchangeOrderKind, ExchangeResponse};

use iced::Task;

// ---------------------------------------------------------------------------
// Quick Trade Market Order Submission
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn handle_submit_quick_trade_order(
        &mut self,
        request: QuickTradeOrderRequest,
    ) -> Task<Message> {
        let Some(instance) = self.charts.get(&request.chart_id) else {
            return self.reject_quick_trade("Quick Trade chart is no longer available");
        };
        if instance.chart.surface_id() != request.surface_id {
            return self.reject_quick_trade("Quick Trade ignored: chart surface changed");
        }
        if instance.symbol != request.symbol_key {
            return self.reject_quick_trade("Quick Trade ignored: chart symbol changed");
        }
        if instance.quick_trade_actions.get(request.action_index) != Some(&request.action) {
            return self.reject_quick_trade("Quick Trade action changed; review it before trading");
        }
        if !request.action.is_valid() {
            return self.reject_quick_trade("Quick Trade action has an invalid quantity");
        }
        if self.reject_if_pending_trading_request("placing a Quick Trade order") {
            self.toast_order_status();
            return Task::none();
        }
        if self.reject_if_account_reconciliation_required(
            "placing a Quick Trade order",
            "account data",
        ) {
            self.toast_order_status();
            return Task::none();
        }

        let Some((key, account_address)) = self.order_signing_context() else {
            self.toast_order_status();
            return Task::none();
        };
        let is_buy = request.action.side.is_buy();
        let intent = PlaceIntent {
            surface: OrderSurface::QuickTrade,
            symbol_key: request.symbol_key,
            is_buy,
            order_kind: ExchangeOrderKind::Market,
            price_source: PriceSource::MarketWithSlippage {
                invalid_message: Some("Invalid Quick Trade market price"),
                usd_size_reference: MarketUsdSizeReference::Mid,
            },
            quantity_source: QuantitySource::UserInput {
                value: request.action.quantity.to_string(),
                denomination: match request.action.denomination {
                    QuickTradeDenomination::Usd => QuantityDenomination::UsdNotional,
                    QuickTradeDenomination::Coin => QuantityDenomination::Coin,
                },
                invalid_message: "Invalid Quick Trade quantity",
                precision_invalid_message: "Quick Trade quantity is below asset precision",
            },
            // Quick Trade actions are explicit directional entries. They do
            // not inherit the unrelated main-ticket reduce-only toggle.
            reduce_only_source: ReduceOnlySource::Fixed(false),
        };
        let prepared = match self.prepare_place_order(intent) {
            Ok(prepared) => prepared,
            Err(message) => return self.reject_quick_trade(message),
        };

        let display_symbol = self.display_name_for_symbol(&prepared.symbol_key);
        self.order_status = Some((
            format!(
                "Placing Quick Trade {} {} {}...",
                request.action.side.label(),
                request.action.quantity_label(),
                display_symbol
            ),
            false,
        ));
        self.pending_order_action = Some(if prepared.is_buy {
            PendingOrderAction::Buy
        } else {
            PendingOrderAction::Sell
        });

        let pending_indicator_id = self.add_pending_market_order_placement_indicator(
            account_address.clone(),
            prepared.symbol_key.clone(),
            prepared.is_buy,
            prepared.size.clone(),
            prepared.price.clone(),
        );
        let market_type = prepared.market_type;
        let (place_request, context) = prepared.place_request_with_context(&account_address);
        self.invalidate_spot_balances_after_exchange_dispatch(&account_address, market_type);

        place_order_task(key, place_request, move |result| {
            Message::QuickTradeOrderResult {
                pending_indicator_id,
                context,
                result: Box::new(result),
            }
        })
    }

    pub(crate) fn handle_quick_trade_order_result(
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

    fn reject_quick_trade(&mut self, message: impl Into<String>) -> Task<Message> {
        self.set_order_status(message.into(), true);
        self.toast_order_status();
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_state::{ChartInstance, ChartSurfaceId};
    use crate::config::{QuickTradeActionConfig, QuickTradeSide};
    use crate::order_execution::QuickTradeOrderRequest;
    use crate::timeframe::Timeframe;

    #[test]
    fn quantity_labels_match_action_denomination() {
        let action = |quantity, denomination| QuickTradeActionConfig {
            side: QuickTradeSide::Buy,
            quantity,
            denomination,
        };
        assert_eq!(
            action(10_000.0, QuickTradeDenomination::Usd).quantity_label(),
            "$10K"
        );
        assert_eq!(
            action(1.0, QuickTradeDenomination::Coin).quantity_label(),
            "1"
        );
        assert_eq!(
            action(0.25, QuickTradeDenomination::Coin).quantity_label(),
            "0.25"
        );
    }

    #[test]
    fn stale_symbol_request_is_rejected_before_signing() {
        let chart_id = 7;
        let surface_id = ChartSurfaceId::Docked(chart_id);
        let action = QuickTradeActionConfig {
            side: QuickTradeSide::Buy,
            quantity: 10_000.0,
            denomination: QuickTradeDenomination::Usd,
        };
        let (mut terminal, _) = TradingTerminal::boot();
        let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
        instance.chart.set_surface_id(surface_id);
        instance.quick_trade_actions.push(action.clone());
        terminal.charts.insert(chart_id, instance);

        let _task = terminal.handle_submit_quick_trade_order(QuickTradeOrderRequest {
            chart_id,
            surface_id,
            symbol_key: "ETH".to_string(),
            action_index: 0,
            action,
        });

        assert!(
            terminal
                .order_status
                .as_ref()
                .is_some_and(|(message, is_error)| *is_error && message.contains("symbol changed"))
        );
        assert!(terminal.pending_order_action.is_none());
    }

    #[test]
    fn changed_action_request_is_rejected_before_signing() {
        let chart_id = 7;
        let surface_id = ChartSurfaceId::Docked(chart_id);
        let configured = QuickTradeActionConfig {
            side: QuickTradeSide::Buy,
            quantity: 10_000.0,
            denomination: QuickTradeDenomination::Usd,
        };
        let (mut terminal, _) = TradingTerminal::boot();
        let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
        instance.chart.set_surface_id(surface_id);
        instance.quick_trade_actions.push(configured);
        terminal.charts.insert(chart_id, instance);

        let _task = terminal.handle_submit_quick_trade_order(QuickTradeOrderRequest {
            chart_id,
            surface_id,
            symbol_key: "BTC".to_string(),
            action_index: 0,
            action: QuickTradeActionConfig {
                side: QuickTradeSide::Sell,
                quantity: 1.0,
                denomination: QuickTradeDenomination::Coin,
            },
        });

        assert!(
            terminal
                .order_status
                .as_ref()
                .is_some_and(|(message, is_error)| *is_error && message.contains("action changed"))
        );
        assert!(terminal.pending_order_action.is_none());
    }
}

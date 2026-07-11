use super::is_fresh_mutation_intent_fenced_during_exit;
use crate::alfred_state::AlfredCommandId;
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartSurfaceId;
use crate::config::OrderPreset;
use crate::message::Message;
use crate::order_execution::{
    AdvancedOrderStartSnapshot, HudOrderRequest, HudOrderSide, HudOrderType,
    OrderLeverageSubmissionSnapshot, QuickOrderForm, QuickOrderSubmissionSnapshot,
    TicketOrderSubmissionSnapshot, TwapOrderStartSnapshot,
};
use crate::signing::OrderKind;
use crate::twap_state::TwapOrderForm;
use crate::wallet_cluster_state::WalletClusterCloseSide;

fn ticket_snapshot() -> TicketOrderSubmissionSnapshot {
    TicketOrderSubmissionSnapshot {
        order_kind: OrderKind::Limit,
        symbol_key: "BTC".to_string(),
        price_input: "100".to_string(),
        quantity_input: "1".to_string(),
        quantity_is_usd: false,
        reduce_only: false,
        market_universe: Default::default(),
    }
}

fn advanced_snapshot() -> AdvancedOrderStartSnapshot {
    AdvancedOrderStartSnapshot {
        order_kind: OrderKind::Chase,
        symbol_key: "BTC".to_string(),
        quantity_input: "1".to_string(),
        quantity_is_usd: false,
        reduce_only: false,
        market_universe: Default::default(),
    }
}

fn twap_snapshot() -> TwapOrderStartSnapshot {
    TwapOrderStartSnapshot {
        order: AdvancedOrderStartSnapshot {
            order_kind: OrderKind::Twap,
            ..advanced_snapshot()
        },
        twap_form: TwapOrderForm::default(),
    }
}

fn quick_snapshot() -> QuickOrderSubmissionSnapshot {
    QuickOrderSubmissionSnapshot {
        surface_id: ChartSurfaceId::Docked(1),
        symbol_key: "BTC".to_string(),
        form: QuickOrderForm {
            price: 100.0,
            quantity: "1".to_string(),
            quantity_is_usd: false,
            percentage: 0.0,
            quantity_provenance: None,
            is_limit: true,
            click_x: 1.0,
            click_y: 1.0,
            chart_w: 100.0,
            chart_h: 100.0,
        },
        reduce_only: false,
        market_universe: Default::default(),
    }
}

fn hud_request() -> HudOrderRequest {
    HudOrderRequest {
        chart_id: 1,
        surface_id: ChartSurfaceId::Docked(1),
        symbol_key: "BTC".to_string(),
        price: 100.0,
        quantity: "1".to_string(),
        order_type: HudOrderType::Limit,
        market_side: HudOrderSide::Long,
        limit_side: Some(HudOrderSide::Long),
        click_x: 1.0,
        click_y: 1.0,
        chart_w: 100.0,
        chart_h: 100.0,
    }
}

#[test]
fn final_exit_fence_classifies_every_fresh_mutation_intent() {
    let messages = vec![
        Message::SubmitOrderLeverage(OrderLeverageSubmissionSnapshot {
            symbol_key: "BTC".to_string(),
            leverage_input: "2".to_string(),
            is_cross: true,
        }),
        Message::ExecutePreset(
            OrderKind::Market,
            OrderPreset {
                label: "$100".to_string(),
                size: 100.0,
                price_offset_pct: None,
            },
            true,
        ),
        Message::PlaceOrder {
            is_buy: true,
            snapshot: ticket_snapshot(),
        },
        Message::ClosePosition {
            coin: "BTC".to_string(),
            fraction: 1.0,
            use_market: true,
        },
        Message::NukePositions,
        Message::StartChase {
            is_buy: true,
            snapshot: advanced_snapshot(),
        },
        Message::StartTwap {
            is_buy: true,
            snapshot: twap_snapshot(),
        },
        Message::SubmitQuickOrder {
            chart_id: 1,
            is_buy: true,
            snapshot: quick_snapshot(),
        },
        Message::SubmitHudOrder(hud_request()),
        Message::MoveOrder {
            coin: "BTC".to_string(),
            oid: 7.into(),
            new_price: 101.0,
        },
        Message::ChaseRestingOrder {
            coin: "BTC".to_string(),
            oid: 7.into(),
        },
        Message::AlfredSubmit,
        Message::AlfredCommandSelected(AlfredCommandId::NaturalLanguageTrading),
        Message::WalletClusterSubmitOrder { is_buy: true },
        Message::WalletClusterClosePosition {
            symbol: "BTC".to_string(),
            side: WalletClusterCloseSide::Long,
            fraction: 1.0,
            use_market: true,
        },
        Message::ClearConfigs,
    ];

    for message in messages {
        assert!(
            is_fresh_mutation_intent_fenced_during_exit(&message),
            "fresh mutation intent was not fenced: {message:?}"
        );
    }
}

#[test]
fn final_exit_fence_allows_reconciliation_and_exposure_cleanup_messages() {
    let messages = vec![
        Message::CancelOrder {
            coin: "BTC".to_string(),
            oid: 7.into(),
        },
        Message::StopChase,
        Message::StopChaseById(1),
        Message::StopAllAdvancedOrders,
        Message::StopTwap(1),
        Message::CancelResult {
            request_id: 1,
            account_address: "0xabc".into(),
            pending_indicator_id: None,
            result: Box::new(Err("transport outcome unknown".to_string())),
        },
        Message::MoveOrderModifyResult {
            request_id: 1,
            account_address: "0xabc".into(),
            coin: "BTC".to_string(),
            oid: 7.into(),
            pending_indicator_id: None,
            result: Box::new(Err("transport outcome unknown".to_string())),
        },
        Message::TwapSliceResult {
            twap_id: 1,
            slice_index: 0,
            retry_count: 0,
            result: Box::new(Err("transport outcome unknown".to_string())),
        },
    ];

    for message in messages {
        assert!(
            !is_fresh_mutation_intent_fenced_during_exit(&message),
            "reconciliation or cleanup message was fenced: {message:?}"
        );
    }
}

#[test]
fn final_exit_fence_drops_mutation_before_its_feature_route_runs() {
    let mut terminal = TradingTerminal::boot().0;
    let move_key = crate::order_execution::MoveOrderKey::new("BTC", 7);
    terminal.active_move_order_drag = Some(move_key.clone());
    terminal.config_save_exit_requested = true;

    let task = terminal.update(Message::MoveOrder {
        coin: "BTC".to_string(),
        oid: 7.into(),
        new_price: 101.0,
    });

    assert_eq!(task.units(), 0);
    assert_eq!(terminal.active_move_order_drag, Some(move_key));
}

#[test]
fn final_exit_fence_drops_new_config_clear_before_its_feature_route_runs() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.config_save_exit_requested = true;

    let task = terminal.update(Message::ClearConfigs);

    assert_eq!(task.units(), 0);
    assert!(!terminal.config_clear_requested);
    assert!(terminal.config_save_exit_requested);
}

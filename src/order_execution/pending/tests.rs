use super::{PendingLeverageUpdateContext, PendingNukeExecution, PendingOrderAction};
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::order_execution::{
    MoveOrderKey, OneShotPlacementContext, OrderSurface, PendingMoveOrderContext,
};
use crate::order_update::{
    PendingCancelStatusRequest, PendingMoveStatusRequest, PendingOneShotStatusRequest,
};
use crate::signing::ExchangeOrderKind;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

#[test]
fn pending_trading_request_tracks_all_account_transition_blockers() {
    let account = TEST_ACCOUNT;
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some(account.to_string());
    assert!(!terminal.has_pending_trading_request());

    terminal.pending_order_action = Some(PendingOrderAction::Buy);
    assert!(terminal.has_pending_trading_request());
    terminal.pending_order_action = None;

    terminal.pending_nuke_execution = Some(PendingNukeExecution::new(1, 1, 0));
    assert!(terminal.has_pending_trading_request());
    terminal.pending_nuke_execution = None;

    terminal.pending_leverage_update = Some(PendingLeverageUpdateContext {
        address: account.to_string(),
        symbol_key: "BTC".to_string(),
        display: "BTC".to_string(),
        asset: 0,
        dex: None,
        is_cross: true,
        leverage: 3,
    });
    assert!(terminal.has_pending_trading_request());
    terminal.pending_leverage_update = None;

    terminal.insert_pending_one_shot_status_request(PendingOneShotStatusRequest::new(
        7,
        &OneShotPlacementContext {
            account_address: account.to_string(),
            cloid: "0x00000000000000000000000000000000".to_string(),
            surface: OrderSurface::Ticket,
            symbol_key: "BTC".to_string(),
            order_kind: ExchangeOrderKind::Limit,
        },
    ));
    assert!(terminal.has_pending_trading_request());
    terminal.pending_one_shot_status_requests.clear();

    terminal.pending_cancel_status_request = Some(PendingCancelStatusRequest::new(
        account.to_string(),
        42,
        "BTC".to_string(),
    ));
    assert!(terminal.has_pending_trading_request());
    terminal.pending_cancel_status_request = None;

    terminal.pending_move_status_request = Some(PendingMoveStatusRequest::new(
        account.to_string(),
        42,
        "BTC".to_string(),
    ));
    assert!(terminal.has_pending_trading_request());
    terminal.pending_move_status_request = None;

    terminal.pending_move_order_contexts.insert(
        MoveOrderKey::new("BTC", 42),
        PendingMoveOrderContext::new(
            account.to_string(),
            sensitive_string("move-agent").into_zeroizing().into(),
        )
        .expect("move context"),
    );
    assert!(terminal.has_pending_trading_request());
    terminal.pending_move_order_contexts.clear();

    let pending_id = terminal.add_pending_order_placement_indicator(
        account.to_string(),
        "BTC".to_string(),
        true,
        "1".to_string(),
        "100".to_string(),
    );
    assert!(pending_id.is_some());
    assert!(terminal.has_pending_trading_request());
}

#[test]
fn hud_overlap_exempts_only_hud_tracking_indicators_and_status_checks() {
    fn assert_hud_allowed(terminal: &mut TradingTerminal) {
        assert!(terminal.reject_if_pending_trading_request("switching accounts"));
        assert_eq!(
            terminal.order_status.as_ref(),
            Some(&(
                "Wait for pending trading requests to finish before switching accounts".to_string(),
                true
            ))
        );
        terminal.order_status = Some(("previous status".to_string(), false));
        assert!(!terminal.reject_if_pending_trading_request_blocking_hud_placement("placing"));
        assert_eq!(
            terminal.order_status.as_ref(),
            Some(&("previous status".to_string(), false))
        );
    }

    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    assert!(!terminal.reject_if_pending_trading_request("switching accounts"));
    assert!(!terminal.reject_if_pending_trading_request_blocking_hud_placement("placing"));
    assert!(terminal.order_status.is_none());

    terminal
        .hud_placements
        .begin(TEST_ACCOUNT.to_string(), None, 1);
    assert_hud_allowed(&mut terminal);
    terminal.hud_placements.clear();

    let indicator_id = terminal
        .add_pending_order_placement_indicator(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            true,
            "1".to_string(),
            "100".to_string(),
        )
        .expect("placement indicator");
    assert!(terminal.reject_if_pending_trading_request_blocking_hud_placement("placing"));
    assert_eq!(
        terminal.order_status.as_ref(),
        Some(&(
            "Wait for pending trading requests to finish before placing".to_string(),
            true
        ))
    );
    terminal
        .hud_placements
        .begin(TEST_ACCOUNT.to_string(), Some(indicator_id), 1);
    assert_hud_allowed(&mut terminal);
    terminal.clear_pending_order_indicator(Some(indicator_id));
    terminal.hud_placements.clear();

    for surface in [OrderSurface::Hud, OrderSurface::Ticket] {
        terminal.insert_pending_one_shot_status_request(PendingOneShotStatusRequest::new(
            7,
            &OneShotPlacementContext {
                account_address: TEST_ACCOUNT.to_string(),
                cloid: "0x00000000000000000000000000000000".to_string(),
                surface,
                symbol_key: "BTC".to_string(),
                order_kind: ExchangeOrderKind::Limit,
            },
        ));
        if surface == OrderSurface::Hud {
            assert_hud_allowed(&mut terminal);
        } else {
            assert!(terminal.reject_if_pending_trading_request_blocking_hud_placement("placing"));
        }
        terminal.pending_one_shot_status_requests.clear();
    }
}

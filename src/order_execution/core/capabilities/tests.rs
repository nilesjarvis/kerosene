use super::*;

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
        OrderSurface::QuickTrade,
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
    assert!(!OrderSurface::Cluster.allows_market_type(OrderOperation::Place, MarketType::Outcome));
    // Cluster closes are reduce-only perp closes only.
    assert!(OrderSurface::ClusterClose.allows_market_type(OrderOperation::Place, MarketType::Perp));
    assert!(
        !OrderSurface::ClusterClose.allows_market_type(OrderOperation::Place, MarketType::Spot)
    );
    assert!(
        !OrderSurface::ClusterClose.allows_market_type(OrderOperation::Place, MarketType::Outcome)
    );
}

#[test]
fn move_and_cancel_keep_outcome_support_for_existing_orders() {
    assert!(OrderSurface::Move.allows_market_type(OrderOperation::Modify, MarketType::Outcome));
    assert!(OrderSurface::Cancel.allows_market_type(OrderOperation::Cancel, MarketType::Outcome));
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
        OrderSurface::Ticket.allows_market_type(OrderOperation::UpdateLeverage, MarketType::Perp)
    );
    assert!(
        !OrderSurface::Ticket.allows_market_type(OrderOperation::UpdateLeverage, MarketType::Spot)
    );
}

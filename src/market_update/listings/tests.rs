use super::*;
use crate::market_state::listings::{ListingMarket, ListingsSnapshot};
use iced::widget::pane_grid;

#[test]
fn listings_stale_response_does_not_finish_current_request() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.listings.loading = true;
    terminal.listings.request_id = 2;
    let _ = terminal.update_listings_market(Message::ListingsLoaded(
        1,
        Box::new(ListingsSnapshot {
            perps: Err("old error".into()),
            spot: Err("old error".into()),
        }),
    ));
    assert!(terminal.listings.loading);
    assert!(terminal.listings.error.is_none());
}

#[test]
fn listings_refresh_requires_open_pane_and_coalesces_requests() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.panes = pane_grid::State::new(PaneKind::Chart(0)).0;
    let _ = terminal.request_listings_refresh(true);
    assert!(!terminal.listings.loading);
    terminal.panes = pane_grid::State::new(PaneKind::NewListings).0;
    let _ = terminal.request_listings_refresh(false);
    assert!(terminal.listings.loading);
    let previous = terminal.listings.last_attempt;
    let _ = terminal.request_listings_refresh(true);
    assert_eq!(terminal.listings.last_attempt, previous);
    terminal.listings.loading = false;
    let _ = terminal.request_listings_refresh(true);
    assert!(
        !terminal.listings.loading,
        "manual refresh is rate limited too"
    );
}

#[test]
fn listings_loaded_seeds_baseline_and_save_failure_is_retried() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.listings.loading = true;
    let snapshot = ListingsSnapshot {
        perps: Ok(vec![ListingMarket {
            id: "perp:BTC".into(),
            key: "BTC".into(),
            label: "BTC".into(),
            active: true,
        }]),
        spot: Err("offline".into()),
    };
    let _ = terminal.update_listings_market(Message::ListingsLoaded(0, Box::new(snapshot)));
    assert!(!terminal.listings.loading);
    assert!(terminal.listings.history.perps.initialized);
    assert!(terminal.listings.history.events.is_empty());
    assert!(terminal.listings.saving);
    let _ = terminal.update_listings_market(Message::ListingsSaved(Err("disk full".into())));
    assert!(terminal.listings.dirty && terminal.listings.storage_error);
    let _ = terminal.save_listings_history();
    assert!(terminal.listings.saving);
    let _ = terminal.update_listings_market(Message::ListingsSaved(Ok(())));
    assert!(!terminal.listings.storage_error && !terminal.listings.dirty);
}

#[test]
fn listings_add_pane_is_singleton_and_renders() {
    let (mut terminal, _) = TradingTerminal::boot();
    let _ = terminal.update_panes(Message::AddNewListingsPane);
    let _ = terminal.update_panes(Message::AddNewListingsPane);
    assert_eq!(
        terminal
            .workspace_pane_kinds()
            .filter(|(_, _, kind)| matches!(kind, PaneKind::NewListings))
            .count(),
        1
    );
    let _view = terminal.view_new_listings();
}

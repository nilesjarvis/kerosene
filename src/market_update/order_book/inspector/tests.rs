use super::*;
use crate::config::KeroseneConfig;
use crate::market_state::{
    OrderBookInstance,
    book_inspector::{InspectorStatus, Page, Selection},
};

fn selection(price: f64) -> Selection {
    Selection {
        price,
        tick: 1.0,
        is_bid: Some(true),
        page: 0,
        revision: 0,
    }
}

fn terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.order_books.clear();
    terminal.active_symbol = "BTC".into();
    terminal.order_books.insert(
        7,
        OrderBookInstance::new(7, OrderBookSymbolMode::Active, 1.0),
    );
    terminal.hydromancer_api_key = "synthetic-key".to_string().into();
    let _ = terminal.update(Message::InspectBookBlock {
        id: 7,
        selection: selection(100.0),
    });
    terminal
}

fn page(revision: u64) -> InspectorEvent {
    InspectorEvent::Page(Page {
        revision,
        height: 10,
        timestamp_ms: 1000,
        orders: vec![],
        total_orders: 0,
        wallets: 0,
        total_size: 0.0,
        notional: 0.0,
        page: 0,
    })
}

#[test]
fn changing_price_reuses_stream_but_rejects_previous_selection_results() {
    let mut terminal = terminal();
    let state = terminal.book_inspector.as_ref().expect("inspector");
    let (id, run, revision) = (state.window_id, state.run_id, state.selection.revision);
    let _ = terminal.update(Message::InspectBookBlock {
        id: 7,
        selection: selection(101.0),
    });
    assert_eq!(
        terminal.book_inspector.as_ref().expect("inspector").run_id,
        run
    );
    let _ = terminal.update(Message::BookInspectorEvent(id, run, 0, page(revision)));
    assert!(
        terminal
            .book_inspector
            .as_ref()
            .expect("inspector")
            .page
            .is_none()
    );
    let _ = terminal.update(Message::BookInspectorEvent(id, run, 0, page(revision + 1)));
    assert!(
        terminal
            .book_inspector
            .as_ref()
            .expect("inspector")
            .page
            .is_some()
    );
}

#[test]
fn inspector_is_pinned_and_market_changes_invalidate_old_streams() {
    let mut terminal = terminal();
    let state = terminal.book_inspector.as_ref().expect("inspector");
    let (id, run) = (state.window_id, state.run_id);
    terminal.active_symbol = "ETH".into();
    assert_eq!(
        terminal.book_inspector.as_ref().expect("inspector").coin,
        "BTC"
    );
    let _ = terminal.update(Message::InspectBookBlock {
        id: 7,
        selection: selection(100.0),
    });
    let state = terminal.book_inspector.as_ref().expect("inspector");
    assert_eq!(state.coin, "ETH");
    assert_eq!(state.window_id, id);
    assert_ne!(state.run_id, run);
    let _ = terminal.update(Message::BookInspectorEvent(id, run, 0, page(1)));
    assert!(
        terminal
            .book_inspector
            .as_ref()
            .expect("inspector")
            .page
            .is_none()
    );
}

#[test]
fn key_rotation_and_window_close_reject_late_results() {
    let mut terminal = terminal();
    let state = terminal.book_inspector.as_ref().expect("inspector");
    let (id, run) = (state.window_id, state.run_id);
    let _ = terminal.update(Message::BookInspectorEvent(id, run, 0, page(0)));
    assert!(
        terminal
            .book_inspector
            .as_ref()
            .expect("inspector")
            .page
            .is_some()
    );
    terminal.bump_hydromancer_key_generation();
    let new_run = terminal.book_inspector.as_ref().expect("inspector").run_id;
    let _ = terminal.update(Message::BookInspectorEvent(id, new_run, 0, page(0)));
    assert!(
        terminal
            .book_inspector
            .as_ref()
            .expect("inspector")
            .page
            .is_none()
    );
    let _ = terminal.update(Message::WindowClosed(id));
    let _ = terminal.update(Message::BookInspectorEvent(id, new_run, 1, page(0)));
    assert!(terminal.book_inspector.is_none());
}

#[test]
fn interruption_marks_retained_rows_stale_until_a_current_page_arrives() {
    let mut terminal = terminal();
    let state = terminal.book_inspector.as_ref().expect("inspector");
    let (id, run) = (state.window_id, state.run_id);
    let _ = terminal.update(Message::BookInspectorEvent(id, run, 0, page(0)));
    let _ = terminal.update(Message::BookInspectorEvent(
        id,
        run,
        0,
        InspectorEvent::Status(InspectorStatus::Recovering("Stream gap".into())),
    ));
    let state = terminal.book_inspector.as_ref().expect("inspector");
    assert!(state.page.is_some());
    assert!(state.status.is_some());
    let _ = terminal.update(Message::BookInspectorEvent(id, run, 0, page(0)));
    assert!(
        terminal
            .book_inspector
            .as_ref()
            .expect("inspector")
            .status
            .is_none()
    );
}

#[test]
fn invalid_or_outdated_price_selections_do_not_open_or_retarget() {
    let mut terminal = terminal();
    for mut selection in [selection(f64::NAN), selection(-1.0), selection(100.0)] {
        if selection.price == 100.0 {
            selection.tick = 10.0;
        }
        let _ = terminal.update(Message::InspectBookBlock { id: 7, selection });
    }
    assert_eq!(
        terminal
            .book_inspector
            .as_ref()
            .expect("inspector")
            .selection,
        selection(100.0)
    );
}

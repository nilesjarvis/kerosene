use super::*;
use crate::api::BookLevel;
use crate::config::KeroseneConfig;
use crate::market_state::OrderBookInstance;

fn depth(tick: f64) -> OrderBook {
    OrderBook {
        bids: (1..=20)
            .map(|i| BookLevel {
                px: 80_000.0 - f64::from(i) * tick,
                sz: 1.0,
            })
            .collect(),
        asks: (1..=20)
            .map(|i| BookLevel {
                px: 80_000.0 + f64::from(i) * tick,
                sz: 2.0,
            })
            .collect(),
    }
}

fn terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.order_books.clear();
    terminal.active_symbol = "BTC".into();
    let mut book = OrderBookInstance::new(7, OrderBookSymbolMode::Active, 1.0);
    book.set_book_with_source(depth(1.0), Some(1.0));
    terminal.order_books.insert(7, book);
    terminal
}

fn deliver(terminal: &mut TradingTerminal, id: u64, tick: f64) {
    let _ = terminal.update_order_book_market(Message::WsBookUpdate {
        id,
        coin: "BTC".into(),
        sigfigs: helpers::compute_sigfigs(tick, 80_000.0),
        source_context: terminal.market_data_source_context(),
        book: depth(tick),
    });
}

#[test]
fn every_denomination_keeps_twenty_rows_through_both_directions() {
    let mut terminal = terminal();
    for tick in [2.0, 5.0, 10.0, 100.0, 10.0, 5.0, 2.0, 1.0] {
        let previous = terminal.order_books[&7].displayed_tick_size();
        let _ = terminal.update_order_book_market(Message::SetBookTickSize(7, tick));
        let inst = &terminal.order_books[&7];
        assert_eq!(inst.displayed_tick_size(), previous);
        assert_eq!(
            inst.aggregated_depth(inst.displayed_tick_size()).bids.len(),
            20
        );
        assert!(inst.book_loading);

        // Queued updates from the old subscription cannot complete this switch.
        deliver(&mut terminal, 7, previous);
        assert!(terminal.order_books[&7].book_loading);
        deliver(&mut terminal, 7, tick);
        let inst = &terminal.order_books[&7];
        assert!(!inst.book_loading);
        assert_eq!(inst.displayed_tick_size(), tick);
        let rows = inst.aggregated_depth(inst.displayed_tick_size());
        assert_eq!(rows.bids.len(), 20);
        assert_eq!(rows.asks.len(), 20);
        assert_eq!(rows.bids.last().map(|row| row.2), Some(20.0));
        assert_eq!(rows.asks.last().map(|row| row.2), Some(40.0));
    }
}

#[test]
fn rapid_switches_reject_old_rest_and_ws_results_and_live_data_wins() {
    let mut terminal = terminal();
    let _ = terminal.update_order_book_market(Message::SetBookTickSize(7, 100.0));
    let old_request = terminal.order_books[&7]
        .pending_book_request_id()
        .expect("request");
    let _ = terminal.update_order_book_market(Message::SetBookTickSize(7, 5.0));
    let request = terminal.order_books[&7]
        .pending_book_request_id()
        .expect("request");
    deliver(&mut terminal, 7, 100.0);
    assert_eq!(terminal.order_books[&7].displayed_tick_size(), 1.0);
    let _ = terminal.apply_order_book_loaded(
        old_request,
        7,
        "BTC".into(),
        100.0,
        (Some(3), None),
        Ok(depth(100.0)),
    );
    assert_eq!(terminal.order_books[&7].displayed_tick_size(), 1.0);
    deliver(&mut terminal, 7, 5.0);
    let live_size = terminal.order_books[&7].book.bids[0].sz;
    let mut older = depth(5.0);
    older.bids[0].sz = 99.0;
    for result in [Ok(older), Err("late request failure".into())] {
        let _ = terminal.apply_order_book_loaded(
            request,
            7,
            "BTC".into(),
            5.0,
            (Some(5), Some(5)),
            result,
        );
        let inst = &terminal.order_books[&7];
        assert_eq!(inst.book.bids[0].sz, live_size);
        assert!(inst.book_error.is_none());
        assert!(!inst.book_loading);
    }
}

#[test]
fn same_symbol_panes_keep_independent_precision_and_automation_stays_fine() {
    let mut terminal = terminal();
    let mut coarse = OrderBookInstance::new(8, OrderBookSymbolMode::Fixed("BTC".into()), 100.0);
    coarse.set_book_with_source(depth(100.0), Some(100.0));
    terminal.order_books.insert(8, coarse);
    assert_eq!(terminal.order_book_sigfigs(7), Some((Some(5), None)));
    assert_eq!(terminal.order_book_sigfigs(8), Some((Some(3), None)));
    assert_eq!(terminal.canonical_l2_book_sigfigs("BTC"), (Some(5), None));
    deliver(&mut terminal, 7, 100.0);
    deliver(&mut terminal, 8, 1.0);
    assert_eq!(terminal.order_books[&7].book.bids[0].px, 79_999.0);
    assert_eq!(terminal.order_books[&8].book.bids[0].px, 79_900.0);
}

#[test]
fn failed_switch_retains_last_depth_and_can_retry_selected_denomination() {
    let mut terminal = terminal();
    let _ = terminal.update_order_book_market(Message::SetBookTickSize(7, 100.0));
    let request = terminal.order_books[&7]
        .pending_book_request_id()
        .expect("request");
    let _ = terminal.apply_order_book_loaded(
        request,
        7,
        "BTC".into(),
        100.0,
        (Some(3), None),
        Err("offline".into()),
    );
    assert_eq!(terminal.order_books[&7].displayed_tick_size(), 1.0);
    assert!(terminal.order_books[&7].book_error.is_some());
    let _ = terminal.update_order_book_market(Message::SetBookTickSize(7, 100.0));
    assert!(terminal.order_books[&7].book_loading);
    deliver(&mut terminal, 7, 100.0);
    assert!(terminal.order_books[&7].book_error.is_none());
    assert!(!terminal.order_books[&7].book_failure_toasted);
}

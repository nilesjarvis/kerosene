use super::*;
use crate::api::{BookLevel, OrderBook};
use crate::app_state::TradingTerminal;

fn terminal_with_fixed_btc_book(id: u64, tick: f64) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    let instance = OrderBookInstance::new(id, OrderBookSymbolMode::Fixed("BTC".to_string()), tick);
    terminal.order_books.insert(id, instance);
    terminal
}

fn populated_book() -> OrderBook {
    OrderBook {
        bids: vec![BookLevel { px: 99.0, sz: 1.0 }],
        asks: vec![BookLevel { px: 101.0, sz: 1.0 }],
    }
}

fn mark_pending_book_request(
    terminal: &mut TradingTerminal,
    id: u64,
    symbol: &str,
    tick_size: f64,
    sigfigs: (Option<u8>, Option<u8>),
) -> u64 {
    let inst = terminal
        .order_books
        .get_mut(&id)
        .expect("order book instance should exist");
    inst.book_loading = true;
    inst.mark_book_request(symbol.to_string(), tick_size, sigfigs)
}

#[test]
fn symbol_switch_reset_clears_pending_request_and_reseeds_tick() {
    let mut terminal = TradingTerminal::boot().0;
    let mut instance = OrderBookInstance::new(9, OrderBookSymbolMode::Active, 10.0);
    // Simulate a fetch in flight for the previous symbol; if this marker
    // survived the switch it would satisfy the dedup guard and silently
    // skip the new symbol's fetch.
    instance.mark_book_request("BTC".to_string(), 10.0, (Some(5), None));
    instance.book_failure_toasted = true;
    terminal.order_books.insert(9, instance);

    terminal.reset_active_order_books_for_symbol("DOGE");

    let inst = terminal
        .order_books
        .get(&9)
        .expect("order book instance should exist");
    assert_eq!(inst.pending_book_sigfigs(), None);
    assert!(!inst.book_failure_toasted);
    assert!(inst.book_loading);
    // No mid is known for the new symbol in a fresh boot, so the tick falls
    // back to the generic default instead of keeping the old symbol's 10.0.
    assert_eq!(inst.tick_size, 0.01);
}

#[test]
fn tick_size_change_replaces_pending_request_when_sigfigs_match() {
    let mut terminal = terminal_with_fixed_btc_book(7, 5.0);
    let now_ms = TradingTerminal::now_ms();
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), now_ms);

    let old_tick = 5.0;
    let new_tick = 6.0;
    let sigfigs = helpers::compute_sigfigs(old_tick, 100.0);
    assert_eq!(helpers::compute_sigfigs(new_tick, 100.0), sigfigs);

    let old_request_id = mark_pending_book_request(&mut terminal, 7, "BTC", old_tick, sigfigs);

    let _ = terminal.update_order_book_market(Message::SetBookTickSize(7, new_tick));
    let inst = terminal
        .order_books
        .get(&7)
        .expect("order book instance should exist");
    assert!(inst.book_loading);
    assert!(inst.pending_book_request_matches("BTC", new_tick, sigfigs));
    assert!(!inst.pending_book_request_matches("BTC", old_tick, sigfigs));
    let new_request_id = inst
        .pending_book_request_id()
        .expect("new request should be pending");
    assert_ne!(old_request_id, new_request_id);

    let _ = terminal.apply_order_book_loaded(
        old_request_id,
        7,
        "BTC".to_string(),
        old_tick,
        sigfigs,
        Ok(populated_book()),
    );
    let inst = terminal
        .order_books
        .get(&7)
        .expect("order book instance should exist");
    assert!(inst.book_loading);
    assert!(inst.pending_book_request_matches("BTC", new_tick, sigfigs));

    let _ = terminal.apply_order_book_loaded(
        new_request_id,
        7,
        "BTC".to_string(),
        new_tick,
        sigfigs,
        Ok(populated_book()),
    );
    let inst = terminal
        .order_books
        .get(&7)
        .expect("order book instance should exist");
    assert!(!inst.book_loading);
    assert_eq!(inst.pending_book_sigfigs(), None);
}

#[test]
fn stale_same_parameter_snapshot_does_not_clear_newer_pending_request() {
    let mut terminal = terminal_with_fixed_btc_book(7, 0.05);
    let sigfigs = (None, None);

    let old_request_id = mark_pending_book_request(&mut terminal, 7, "BTC", 0.05, sigfigs);
    let new_request_id = mark_pending_book_request(&mut terminal, 7, "BTC", 0.05, sigfigs);
    assert_ne!(old_request_id, new_request_id);

    let _ = terminal.apply_order_book_loaded(
        old_request_id,
        7,
        "BTC".to_string(),
        0.05,
        sigfigs,
        Ok(populated_book()),
    );
    let inst = terminal
        .order_books
        .get(&7)
        .expect("order book instance should exist");
    assert!(inst.book_loading);
    assert_eq!(inst.pending_book_request_id(), Some(new_request_id));
    assert!(inst.book.bids.is_empty());
    assert!(inst.book.asks.is_empty());

    let _ = terminal.apply_order_book_loaded(
        new_request_id,
        7,
        "BTC".to_string(),
        0.05,
        sigfigs,
        Ok(populated_book()),
    );
    let inst = terminal
        .order_books
        .get(&7)
        .expect("order book instance should exist");
    assert!(!inst.book_loading);
    assert_eq!(inst.pending_book_sigfigs(), None);
    assert_eq!(inst.book.bids.len(), 1);
    assert_eq!(inst.book.asks.len(), 1);
}

#[test]
fn repeated_load_failures_toast_once_per_streak() {
    let mut terminal = terminal_with_fixed_btc_book(7, 50.0);
    let toasts_before = terminal.toasts.len();

    let request_id = mark_pending_book_request(&mut terminal, 7, "BTC", 50.0, (None, None));
    let _ = terminal.apply_order_book_loaded(
        request_id,
        7,
        "BTC".to_string(),
        50.0,
        (None, None),
        Err("rate limited".to_string()),
    );
    // A live WS frame clears the inline error between retries; the toast
    // streak flag must survive that, or every retry toasts again.
    if let Some(inst) = terminal.order_books.get_mut(&7) {
        inst.book_error = None;
    }
    let request_id = mark_pending_book_request(&mut terminal, 7, "BTC", 50.0, (None, None));
    let _ = terminal.apply_order_book_loaded(
        request_id,
        7,
        "BTC".to_string(),
        50.0,
        (None, None),
        Err("rate limited".to_string()),
    );

    assert_eq!(terminal.toasts.len(), toasts_before + 1);
    let inst = terminal
        .order_books
        .get(&7)
        .expect("order book instance should exist");
    assert!(inst.book_error.is_some());
    assert!(inst.book_failure_toasted);
}

#[test]
fn successful_load_ends_the_failure_streak() {
    // Tick chosen to sit inside the populated book's option set, so the
    // success path does not re-seed it and later loads still match.
    let mut terminal = terminal_with_fixed_btc_book(7, 0.05);

    let request_id = mark_pending_book_request(&mut terminal, 7, "BTC", 0.05, (None, None));
    let _ = terminal.apply_order_book_loaded(
        request_id,
        7,
        "BTC".to_string(),
        0.05,
        (None, None),
        Err("rate limited".to_string()),
    );
    let request_id = mark_pending_book_request(&mut terminal, 7, "BTC", 0.05, (None, None));
    let _ = terminal.apply_order_book_loaded(
        request_id,
        7,
        "BTC".to_string(),
        0.05,
        (None, None),
        Ok(populated_book()),
    );

    let inst = terminal
        .order_books
        .get(&7)
        .expect("order book instance should exist");
    assert!(inst.book_error.is_none());
    assert!(!inst.book_failure_toasted);

    // The next failure streak toasts again.
    let toasts_before = terminal.toasts.len();
    let request_id = mark_pending_book_request(&mut terminal, 7, "BTC", 0.05, (None, None));
    let _ = terminal.apply_order_book_loaded(
        request_id,
        7,
        "BTC".to_string(),
        0.05,
        (None, None),
        Err("rate limited".to_string()),
    );
    assert_eq!(terminal.toasts.len(), toasts_before + 1);
}

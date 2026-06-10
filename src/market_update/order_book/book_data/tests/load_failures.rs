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

#[test]
fn symbol_switch_reset_clears_pending_request_and_reseeds_tick() {
    let mut terminal = TradingTerminal::boot().0;
    let mut instance = OrderBookInstance::new(9, OrderBookSymbolMode::Active, 10.0);
    // Simulate a fetch in flight for the previous symbol; if this marker
    // survived the switch it would satisfy the dedup guard and silently
    // skip the new symbol's fetch.
    instance.mark_book_request((Some(5), None));
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
fn repeated_load_failures_toast_once_per_streak() {
    let mut terminal = terminal_with_fixed_btc_book(7, 50.0);
    let toasts_before = terminal.toasts.len();

    let _ = terminal.apply_order_book_loaded(
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
    let _ = terminal.apply_order_book_loaded(
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

    let _ = terminal.apply_order_book_loaded(
        7,
        "BTC".to_string(),
        0.05,
        (None, None),
        Err("rate limited".to_string()),
    );
    let _ = terminal.apply_order_book_loaded(
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
    let _ = terminal.apply_order_book_loaded(
        7,
        "BTC".to_string(),
        0.05,
        (None, None),
        Err("rate limited".to_string()),
    );
    assert_eq!(terminal.toasts.len(), toasts_before + 1);
}

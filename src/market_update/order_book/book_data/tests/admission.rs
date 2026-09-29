use super::*;
use crate::api::BookLevel;
use crate::config::KeroseneConfig;

fn instance(symbol: &str) -> OrderBookInstance {
    let mut inst = OrderBookInstance::new(7, OrderBookSymbolMode::Fixed(symbol.to_string()), 0.5);
    inst.set_book(OrderBook {
        bids: vec![BookLevel { px: 99.0, sz: 1.0 }],
        asks: vec![BookLevel { px: 101.0, sz: 2.0 }],
    });
    inst.book_loading = true;
    inst.book_error = Some("previous error".to_string());
    inst.book_failure_toasted = true;
    inst
}

#[test]
fn rejected_fetches_clear_pending_work_but_retain_cached_book_and_failure_streak() {
    for (symbol, muted, reason) in [
        ("", false, "No order-book symbol selected"),
        (
            "BTC",
            true,
            "Order book ticker is hidden in Settings > Risk",
        ),
    ] {
        let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
        terminal.order_books.clear();
        if muted {
            terminal.muted_tickers.insert(symbol.to_string());
        }
        let mut inst = instance(symbol);
        inst.mark_book_request(symbol.to_string(), 0.5, (Some(4), None));
        terminal.order_books.insert(7, inst);
        let toasts = terminal.toasts.len();
        let _ = terminal.order_book_fetch_task_for_id(99);
        assert_eq!(terminal.order_books.len(), 1);
        let _ = terminal.order_book_fetch_task_for_id(7);
        let inst = terminal.order_books.get(&7).expect("book");
        assert!(!inst.book_loading);
        assert_eq!(inst.pending_book_request_id(), None);
        assert_eq!(inst.book_error.as_deref(), Some(reason));
        assert!(inst.book_failure_toasted);
        assert_eq!(inst.book.bids.first().map(|level| level.px), Some(99.0));
        assert_eq!(inst.book.asks.first().map(|level| level.px), Some(101.0));
        assert_eq!(terminal.toasts.len(), toasts);
    }
}

#[test]
fn admission_coalesces_only_loading_matches_and_owns_the_current_mode_symbol() {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.order_books.clear();
    let sigfigs = helpers::compute_sigfigs(0.5, 100.0);
    let mut inst = instance("BTC");
    let first = inst.mark_book_request("BTC".to_string(), 0.5, sigfigs);
    terminal.order_books.insert(7, inst);

    let _ = terminal.order_book_fetch_task_for_id(7);
    let inst = terminal.order_books.get_mut(&7).expect("book");
    assert_eq!(inst.pending_book_request_id(), Some(first));
    assert_eq!(inst.book_error.as_deref(), Some("previous error"));
    assert!(inst.book_failure_toasted);
    inst.book_loading = false;

    let _ = terminal.order_book_fetch_task_for_id(7);
    let inst = terminal.order_books.get_mut(&7).expect("book");
    assert_eq!(inst.pending_book_request_id(), Some(first + 1));
    assert!(inst.pending_book_request_matches("BTC", 0.5, sigfigs));
    assert!(inst.book_loading && inst.book_failure_toasted);
    assert_eq!(inst.book_error, None);
    inst.mode = OrderBookSymbolMode::Active;
    terminal.active_symbol = "ETH".to_string();

    let _ = terminal.order_book_fetch_task_for_id(7);
    terminal.active_symbol = "SOL".to_string();
    let inst = terminal.order_books.get(&7).expect("book");
    assert_eq!(inst.pending_book_request_id(), Some(first + 2));
    assert!(inst.pending_book_request_matches("ETH", 0.5, sigfigs));
    assert!(!inst.pending_book_request_matches("SOL", 0.5, sigfigs));
    assert_eq!(inst.book.mid_price(), 100.0);
}

use super::fixtures::{test_twap, twap_by_id};
use crate::api::{BookLevel, OrderBook};
use crate::app_state::TradingTerminal;
use crate::config::ReadDataProvider;
use crate::twap_state::{
    TwapBookSnapshot, TwapChildStatus, TwapPauseReason, TwapPendingOp, TwapStatus,
};

use std::time::Duration;
use std::time::Instant;

fn source_context(
    terminal: &TradingTerminal,
    hydromancer_key_generation: Option<u64>,
) -> crate::read_data_provider::MarketDataSourceContext {
    crate::read_data_provider::MarketDataSourceContext {
        hydromancer_key_generation,
        ..terminal.market_data_source_context()
    }
}

#[test]
fn advanced_exchange_requests_pause_while_account_reconciliation_is_loading() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal.account_loading = false;

    assert!(terminal.can_send_advanced_exchange_request(now));

    terminal.account_loading = true;

    assert!(!terminal.can_send_advanced_exchange_request(now));

    terminal.account_loading = false;
    terminal.account_reconciliation_required = true;

    assert!(!terminal.can_send_advanced_exchange_request(now));
}

#[test]
fn twap_deadline_waits_for_pending_status_reconciliation() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    let mut status_check_twap = test_twap(1, "0xaaa", now);
    status_check_twap.ends_at = now - Duration::from_secs(1);
    status_check_twap.child_orders[0].status = TwapChildStatus::NoFill;
    terminal.twap_orders.insert(1, status_check_twap);

    assert!(!terminal.expire_twap_if_deadline_passed(1, now));
    assert_eq!(twap_by_id(&terminal, 1).status, TwapStatus::Paused);

    let mut unknown_child_twap = test_twap(2, "0xbbb", now);
    unknown_child_twap.ends_at = now - Duration::from_secs(1);
    unknown_child_twap.status_check_cloid = None;
    unknown_child_twap.child_orders[0].status = TwapChildStatus::StatusUnknown;
    terminal.twap_orders.insert(2, unknown_child_twap);

    assert!(!terminal.expire_twap_if_deadline_passed(2, now));
    assert_eq!(twap_by_id(&terminal, 2).status, TwapStatus::Paused);
}

#[test]
fn expired_twap_waiting_for_status_does_not_starve_due_twap() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc".to_string());
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;

    let mut stuck_twap = test_twap(1, "0xaaa", now);
    stuck_twap.ends_at = now - Duration::from_secs(1);
    stuck_twap.status_check_cloid = Some("0xaaa".to_string());
    terminal.twap_orders.insert(1, stuck_twap);

    let mut due_twap = test_twap(2, "0xbbb", now);
    due_twap.status = TwapStatus::Running;
    due_twap.pause_reason = None;
    due_twap.status_check_cloid = None;
    due_twap.child_orders.clear();
    due_twap.next_slice_due = now - Duration::from_secs(1);
    due_twap.latest_book = Some(TwapBookSnapshot {
        book: book(99.0, 101.0),
        updated_at: now,
    });
    terminal.twap_orders.insert(2, due_twap);

    let _task = terminal.handle_twap_tick();

    assert_eq!(twap_by_id(&terminal, 1).status, TwapStatus::Paused);
    assert!(matches!(
        twap_by_id(&terminal, 2).pending_op,
        Some(TwapPendingOp::Place(_))
    ));
}

#[test]
fn twap_book_lag_clears_cached_book_and_pauses_until_fresh_update() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    let mut twap = test_twap(1, "0xaaa", now);
    twap.status = TwapStatus::Running;
    twap.pause_reason = None;
    twap.status_check_cloid = None;
    terminal.twap_orders.insert(1, twap);

    let sigfigs = terminal.canonical_l2_book_sigfigs("BTC");
    let _task = terminal.handle_twap_book_update(
        1,
        "BTC".to_string(),
        sigfigs,
        source_context(&terminal, None),
        book(99.0, 101.0),
    );
    assert!(twap_by_id(&terminal, 1).latest_book.is_some());

    let _task = terminal.handle_twap_book_lagged(
        1,
        "BTC".to_string(),
        sigfigs,
        source_context(&terminal, None),
        4,
    );

    let twap = twap_by_id(&terminal, 1);
    assert!(twap.latest_book.is_none());
    assert_eq!(twap.status, TwapStatus::Paused);
    assert_eq!(twap.pause_reason, Some(TwapPauseReason::StaleMarketData));
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| *is_error && message.contains("market data lagged"))
    );

    let _task = terminal.handle_twap_book_update(
        1,
        "BTC".to_string(),
        sigfigs,
        source_context(&terminal, None),
        book(100.0, 102.0),
    );

    let twap = twap_by_id(&terminal, 1);
    assert!(twap.latest_book.is_some());
    assert_eq!(twap.status, TwapStatus::Running);
    assert_eq!(twap.pause_reason, None);
}

#[test]
fn twap_book_update_ignores_stale_hydromancer_generation() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    terminal.hydromancer_key_generation = 2;
    let mut twap = test_twap(1, "0xaaa", now);
    twap.status = TwapStatus::WaitingForMarket;
    twap.pause_reason = None;
    twap.status_check_cloid = None;
    terminal.twap_orders.insert(1, twap);

    let sigfigs = terminal.canonical_l2_book_sigfigs("BTC");
    let _task = terminal.handle_twap_book_update(
        1,
        "BTC".to_string(),
        sigfigs,
        source_context(&terminal, Some(1)),
        book(99.0, 101.0),
    );

    let twap = twap_by_id(&terminal, 1);
    assert!(twap.latest_book.is_none());
    assert_eq!(twap.status, TwapStatus::WaitingForMarket);

    let _task = terminal.handle_twap_book_update(
        1,
        "BTC".to_string(),
        sigfigs,
        source_context(&terminal, Some(2)),
        book(100.0, 102.0),
    );

    let twap = twap_by_id(&terminal, 1);
    assert!(twap.latest_book.is_some());
    assert_eq!(twap.status, TwapStatus::Running);
}

#[test]
fn twap_book_update_ignores_stale_hyperliquid_generation() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    let mut twap = test_twap(1, "0xaaa", now);
    twap.status = TwapStatus::WaitingForMarket;
    twap.pause_reason = None;
    twap.status_check_cloid = None;
    terminal.twap_orders.insert(1, twap);

    let sigfigs = terminal.canonical_l2_book_sigfigs("BTC");
    let stale_context = source_context(&terminal, None);
    terminal.bump_read_data_provider_generation();
    let _task = terminal.handle_twap_book_update(
        1,
        "BTC".to_string(),
        sigfigs,
        stale_context,
        book(99.0, 101.0),
    );

    let twap = twap_by_id(&terminal, 1);
    assert!(twap.latest_book.is_none());
    assert_eq!(twap.status, TwapStatus::WaitingForMarket);
}

#[test]
fn twap_book_update_ignores_inactive_provider_source() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal.hydromancer_key_generation = 2;
    let mut twap = test_twap(1, "0xaaa", now);
    twap.status = TwapStatus::WaitingForMarket;
    terminal.twap_orders.insert(1, twap);

    let sigfigs = terminal.canonical_l2_book_sigfigs("BTC");
    let _task = terminal.handle_twap_book_update(
        1,
        "BTC".to_string(),
        sigfigs,
        source_context(&terminal, Some(2)),
        book(99.0, 101.0),
    );

    let twap = twap_by_id(&terminal, 1);
    assert!(twap.latest_book.is_none());
    assert_eq!(twap.status, TwapStatus::WaitingForMarket);

    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    let _task = terminal.handle_twap_book_update(
        1,
        "BTC".to_string(),
        sigfigs,
        source_context(&terminal, None),
        book(100.0, 102.0),
    );

    let twap = twap_by_id(&terminal, 1);
    assert!(twap.latest_book.is_none());
    assert_eq!(twap.status, TwapStatus::WaitingForMarket);
}

#[test]
fn twap_book_lag_ignores_stale_hydromancer_generation() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    terminal.hydromancer_key_generation = 2;
    let mut twap = test_twap(1, "0xaaa", now);
    twap.status = TwapStatus::Running;
    twap.pause_reason = None;
    twap.status_check_cloid = None;
    twap.latest_book = Some(TwapBookSnapshot {
        book: book(99.0, 101.0),
        updated_at: now,
    });
    terminal.twap_orders.insert(1, twap);

    let sigfigs = terminal.canonical_l2_book_sigfigs("BTC");
    let _task = terminal.handle_twap_book_lagged(
        1,
        "BTC".to_string(),
        sigfigs,
        source_context(&terminal, Some(1)),
        4,
    );

    let twap = twap_by_id(&terminal, 1);
    assert!(twap.latest_book.is_some());
    assert_eq!(twap.status, TwapStatus::Running);
    assert_eq!(twap.pause_reason, None);

    let _task = terminal.handle_twap_book_lagged(
        1,
        "BTC".to_string(),
        sigfigs,
        source_context(&terminal, Some(2)),
        4,
    );

    let twap = twap_by_id(&terminal, 1);
    assert!(twap.latest_book.is_none());
    assert_eq!(twap.status, TwapStatus::Paused);
    assert_eq!(twap.pause_reason, Some(TwapPauseReason::StaleMarketData));
}

#[test]
fn twap_book_update_ignores_noncanonical_sigfigs() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    let mut twap = test_twap(1, "0xaaa", now);
    twap.status = TwapStatus::Running;
    twap.pause_reason = None;
    twap.status_check_cloid = None;
    terminal.twap_orders.insert(1, twap);

    let _task = terminal.handle_twap_book_update(
        1,
        "BTC".to_string(),
        (Some(5), None),
        source_context(&terminal, None),
        book(99.0, 101.0),
    );

    let twap = twap_by_id(&terminal, 1);
    assert!(twap.latest_book.is_none());
    assert_eq!(twap.status, TwapStatus::Running);
}

#[test]
fn twap_book_lag_ignores_noncanonical_sigfigs() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    let mut twap = test_twap(1, "0xaaa", now);
    twap.status = TwapStatus::Running;
    twap.pause_reason = None;
    twap.status_check_cloid = None;
    twap.latest_book = Some(TwapBookSnapshot {
        book: book(99.0, 101.0),
        updated_at: now,
    });
    terminal.twap_orders.insert(1, twap);

    let _task = terminal.handle_twap_book_lagged(
        1,
        "BTC".to_string(),
        (Some(5), None),
        source_context(&terminal, None),
        4,
    );

    let twap = twap_by_id(&terminal, 1);
    assert!(twap.latest_book.is_some());
    assert_eq!(twap.status, TwapStatus::Running);
    assert_eq!(twap.pause_reason, None);
}

fn book(best_bid: f64, best_ask: f64) -> OrderBook {
    OrderBook {
        bids: vec![BookLevel {
            px: best_bid,
            sz: 1.0,
        }],
        asks: vec![BookLevel {
            px: best_ask,
            sz: 1.0,
        }],
    }
}

use super::super::{OrderBookInstance, OrderBookSymbolMode};
use super::book_at_mid;

use std::time::{Duration, Instant};

#[test]
fn short_term_price_move_tracks_recent_mid_delta() {
    let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 1.0);
    let now = Instant::now();

    inst.set_book(book_at_mid(100.0));
    inst.record_mid_price_sample(now);
    inst.set_book(book_at_mid(101.25));
    inst.record_mid_price_sample(now + Duration::from_secs(3));

    assert_eq!(inst.short_term_price_move(), Some(1.25));
}

#[test]
fn short_term_price_move_uses_oldest_sample_inside_three_second_window() {
    let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 1.0);
    let now = Instant::now();

    inst.set_book(book_at_mid(100.0));
    inst.record_mid_price_sample(now);
    inst.set_book(book_at_mid(101.0));
    inst.record_mid_price_sample(now + Duration::from_secs(2));
    inst.set_book(book_at_mid(102.5));
    inst.record_mid_price_sample(now + Duration::from_secs(5));

    assert_eq!(inst.short_term_price_move(), Some(1.5));
}

#[test]
fn unchanged_mid_samples_update_time_without_growing_history() {
    let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 1.0);
    let now = Instant::now();
    let later = now + Duration::from_secs(3);

    inst.set_book(book_at_mid(100.0));
    inst.record_mid_price_sample(now);
    inst.record_mid_price_sample(later);

    assert_eq!(inst.mid_price_history.len(), 1);
    assert_eq!(
        inst.mid_price_history.front().copied(),
        Some((later, 100.0))
    );
    assert_eq!(inst.short_term_price_move(), None);
}

#[test]
fn clearing_mid_price_history_removes_short_term_move() {
    let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 1.0);
    let now = Instant::now();

    inst.set_book(book_at_mid(100.0));
    inst.record_mid_price_sample(now);
    inst.set_book(book_at_mid(99.5));
    inst.record_mid_price_sample(now + Duration::from_secs(2));

    inst.clear_mid_price_history();

    assert_eq!(inst.short_term_price_move(), None);
}

use super::allocate_exchange_nonce_from;
use std::sync::atomic::{AtomicU64, Ordering};

#[test]
fn exchange_nonce_allocator_is_monotonic_inside_same_millisecond() {
    let last_nonce = AtomicU64::new(0);

    let first = allocate_exchange_nonce_from(&last_nonce, 1_700_000_000_000);
    let second = allocate_exchange_nonce_from(&last_nonce, 1_700_000_000_000);
    let third = allocate_exchange_nonce_from(&last_nonce, 1_700_000_000_000);

    assert_eq!(first, 1_700_000_000_000);
    assert_eq!(second, first + 1);
    assert_eq!(third, second + 1);
}

#[test]
fn exchange_nonce_allocator_never_moves_backwards_when_clock_regresses() {
    let last_nonce = AtomicU64::new(5_000);

    let nonce = allocate_exchange_nonce_from(&last_nonce, 4_000);

    assert_eq!(nonce, 5_001);
    assert_eq!(last_nonce.load(Ordering::SeqCst), 5_001);
}

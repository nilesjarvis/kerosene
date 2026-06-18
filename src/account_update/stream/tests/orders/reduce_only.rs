use super::{open_order, preserve_open_order_reduce_only};

#[test]
fn websocket_open_order_preserves_known_reduce_only_metadata_when_omitted() {
    let existing = vec![open_order(42, Some(true))];
    let mut incoming = open_order(42, None);

    preserve_open_order_reduce_only(&mut incoming, &existing);

    assert_eq!(incoming.reduce_only, Some(true));
}

#[test]
fn websocket_open_order_keeps_unknown_reduce_only_for_new_orders() {
    let existing = vec![open_order(42, Some(true))];
    let mut incoming = open_order(43, None);

    preserve_open_order_reduce_only(&mut incoming, &existing);

    assert_eq!(incoming.reduce_only, None);
}

#[test]
fn websocket_open_order_does_not_copy_reduce_only_across_symbols() {
    let existing = vec![open_order(42, Some(true))];
    let mut incoming = open_order(42, None);
    incoming.coin = "ETH".to_string();

    preserve_open_order_reduce_only(&mut incoming, &existing);

    assert_eq!(incoming.reduce_only, None);
}

#[test]
fn websocket_open_order_keeps_explicit_reduce_only_metadata() {
    let existing = vec![open_order(42, Some(true))];
    let mut incoming = open_order(42, Some(false));

    preserve_open_order_reduce_only(&mut incoming, &existing);

    assert_eq!(incoming.reduce_only, Some(false));
}

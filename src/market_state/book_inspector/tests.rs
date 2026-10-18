use super::*;
use crate::api::BookLevel;

fn order(oid: u64, bid: bool, price: &str, size: &str) -> RestingOrder {
    RestingOrder {
        oid,
        wallet: format!("0x{:040x}", oid % 3).into(),
        is_bid: bid,
        price: parse_decimal(price).expect("price"),
        size: parse_decimal(size).expect("size"),
    }
}

fn selection(price: f64, tick: f64, side: Option<bool>) -> Selection {
    Selection {
        price,
        tick,
        is_bid: side,
        page: 0,
        revision: 1,
    }
}

fn book(orders: Vec<RestingOrder>) -> Book {
    Book::from_snapshot(Snapshot {
        height: 10,
        timestamp_ms: 1000,
        orders,
    })
    .expect("snapshot")
}

fn apply(book: &mut Book, height: u64, diffs: Vec<Diff>) {
    assert!(
        book.apply(Block {
            height,
            timestamp_ms: 1000 + height,
            diffs
        })
        .is_ok()
    );
}

fn ids(book: &Book, side: Option<bool>) -> Vec<u64> {
    let mut ids: Vec<_> = book
        .page(selection(100.0, 100.0, side))
        .orders
        .iter()
        .map(|o| o.oid)
        .collect();
    ids.sort();
    ids
}

#[test]
fn decimal_parsing_is_exact_and_rejects_rounding_and_overflow() {
    for text in [
        "0",
        "0.00000001",
        "63.239",
        "100.12345678",
        "184467440737.09551615",
    ] {
        let parsed = parse_decimal(text).expect("valid decimal");
        assert_eq!(parse_decimal(&format_decimal(parsed)), Ok(parsed));
    }
    assert_eq!(parse_decimal("1.2300000000"), Ok(123_000_000));
    for text in [
        "-1",
        "NaN",
        "inf",
        "1e3",
        "1.000000001",
        "184467440737.09551616",
        "",
        ".1",
        "1.1.1",
        " 1",
    ] {
        assert!(parse_decimal(text).is_err());
    }
}

#[test]
fn inspector_uses_the_same_side_specific_buckets_as_the_depth_widget() {
    for tick in [0.001, 0.01, 0.05, 1.0, 10.0] {
        for bid in [true, false] {
            for price in ["63.239", "63.24", "63.241", "63.25", "100", "99.99999999"] {
                let order = order(1, bid, price, "1");
                let level = BookLevel {
                    px: order.price as f64 / SCALE as f64,
                    sz: 1.0,
                };
                let aggregated = crate::helpers::aggregate_levels(&[level], tick, bid);
                assert!(selection(aggregated[0].0, tick, Some(bid)).contains(&order));
                assert!(!selection(aggregated[0].0, tick, Some(!bid)).contains(&order));
                assert!(!selection(aggregated[0].0 + tick, tick, Some(bid)).contains(&order));
            }
        }
    }
}

#[test]
fn boundary_orders_stay_on_the_correct_side_of_the_block() {
    let bid = selection(100.0, 1.0, Some(true));
    assert!(bid.contains(&order(1, true, "100", "1")));
    assert!(bid.contains(&order(1, true, "100.5", "1")));
    assert!(!bid.contains(&order(1, true, "101", "1")));
    let ask = selection(100.0, 1.0, Some(false));
    assert!(ask.contains(&order(1, false, "100", "1")));
    assert!(ask.contains(&order(1, false, "99.5", "1")));
    assert!(!ask.contains(&order(1, false, "99", "1")));
}

#[test]
fn matching_consumes_price_time_queues_and_rests_only_the_remainder() {
    let mut book = book(vec![
        order(1, false, "100", "2"),
        order(2, false, "100", "3"),
        order(3, false, "101", "4"),
    ]);
    apply(
        &mut book,
        11,
        vec![Diff::New(order(4, true, "101", "6"), None)],
    );
    let page = book.page(selection(200.0, 200.0, Some(false)));
    assert_eq!(page.orders.len(), 1);
    assert_eq!(page.orders[0].oid, 3);
    assert_eq!(page.orders[0].size, 3 * SCALE);
    apply(
        &mut book,
        12,
        vec![Diff::New(order(5, true, "101", "5"), None)],
    );
    let page = book.page(selection(100.0, 100.0, Some(true)));
    assert_eq!(page.orders.len(), 1);
    assert_eq!(page.orders[0].oid, 5);
    assert_eq!(page.orders[0].size, 2 * SCALE);
    assert!(
        book.page(selection(200.0, 200.0, Some(false)))
            .orders
            .is_empty()
    );
}

#[test]
fn priority_insertion_changes_which_wallet_is_filled_first() {
    let mut book = book(vec![order(1, true, "100", "1"), order(2, true, "100", "1")]);
    apply(
        &mut book,
        11,
        vec![Diff::New(order(3, true, "100", "1"), Some(1))],
    );
    apply(
        &mut book,
        12,
        vec![Diff::New(order(4, false, "100", "1"), None)],
    );
    assert_eq!(ids(&book, Some(true)), vec![1, 2]);
    apply(
        &mut book,
        13,
        vec![
            Diff::Remove(1),
            Diff::New(order(5, true, "100", "1"), Some(999)),
        ],
    );
    apply(
        &mut book,
        14,
        vec![Diff::New(order(6, false, "100", "1"), None)],
    );
    assert_eq!(ids(&book, Some(true)), vec![5]);
}

#[test]
fn removals_and_size_updates_preserve_queue_links() {
    let mut book = book((1..=5).map(|id| order(id, true, "100", "1")).collect());
    apply(
        &mut book,
        11,
        vec![
            Diff::Remove(3),
            Diff::Update(1, 0),
            Diff::Remove(5),
            Diff::Update(2, 2 * SCALE),
        ],
    );
    assert_eq!(ids(&book, Some(true)), vec![2, 4]);
    apply(
        &mut book,
        12,
        vec![
            Diff::New(order(6, false, "100", "2.5"), None),
            Diff::Remove(999),
            Diff::Update(999, 1),
        ],
    );
    let page = book.page(selection(100.0, 100.0, Some(true)));
    assert_eq!(page.orders.len(), 1);
    assert_eq!(page.orders[0].oid, 4);
    assert_eq!(page.orders[0].size, SCALE / 2);
}

#[test]
fn gaps_and_backwards_timestamps_require_a_fresh_baseline() {
    let mut book = book(vec![order(1, true, "100", "1")]);
    assert!(
        book.apply(Block {
            height: 12,
            timestamp_ms: 1012,
            diffs: vec![Diff::Remove(1)]
        })
        .is_err()
    );
    assert_eq!(book.height(), 10);
    assert!(
        book.apply(Block {
            height: 11,
            timestamp_ms: 999,
            diffs: vec![]
        })
        .is_err()
    );
    apply(&mut book, 9, vec![Diff::Remove(1)]);
    apply(&mut book, 10, vec![Diff::Remove(1)]);
    assert_eq!(ids(&book, Some(true)), vec![1]);
    apply(&mut book, 11, vec![Diff::Update(1, 2 * SCALE)]);
    apply(&mut book, 11, vec![Diff::Remove(1)]);
    assert_eq!(ids(&book, Some(true)), vec![1]);
}

#[test]
fn duplicate_or_crossed_snapshots_are_rejected() {
    for orders in [
        vec![order(1, true, "100", "1"), order(1, true, "100", "1")],
        vec![order(1, true, "100", "1"), order(2, false, "100", "1")],
    ] {
        assert!(
            Book::from_snapshot(Snapshot {
                height: 10,
                timestamp_ms: 1000,
                orders
            })
            .is_err()
        );
    }
}

#[test]
fn pages_are_bounded_stable_and_report_totals_for_the_entire_block() {
    let book = book(
        (1..=123)
            .rev()
            .map(|id| order(id, true, "100", "1"))
            .collect(),
    );
    let mut selection = selection(100.0, 1.0, Some(true));
    let first = book.page(selection);
    assert_eq!(first.orders.len(), PAGE_SIZE);
    assert_eq!(first.wallets, 3);
    assert_eq!(first.total_orders, 123);
    assert_eq!(first.total_size, 123.0);
    assert_eq!(first.notional, 12300.0);
    assert!(
        first
            .orders
            .windows(2)
            .all(|pair| pair[0].wallet.as_str() <= pair[1].wallet.as_str())
    );
    selection.page = usize::MAX;
    let last = book.page(selection);
    assert_eq!(last.page, 2);
    assert_eq!(last.orders.len(), 23);
}

#[test]
fn retargeting_keeps_the_stream_and_invalidates_the_displayed_page() {
    let mut state = BookInspectorState::new(
        iced::window::Id::unique(),
        "BTC".into(),
        selection(100.0, 1.0, Some(true)),
    );
    state.select(selection(101.0, 1.0, Some(false)));
    assert_eq!(state.run_id, 0);
    assert_eq!(state.selection.revision, 2);
    assert_eq!(*state.selection_rx.borrow(), state.selection);
    state.restart();
    assert_eq!(state.run_id, 1);
    assert!(state.page.is_none());
}

#[test]
fn order_and_page_debug_do_not_leak_wallets() {
    let page = book(vec![order(1, true, "100", "1")]).page(selection(100.0, 1.0, Some(true)));
    assert!(!format!("{page:?}").contains(page.orders[0].wallet.as_str()));
}

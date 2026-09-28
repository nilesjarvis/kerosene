use crate::account::WalletTrackerSnapshot;
use crate::app_state::TradingTerminal;
use crate::config::KeroseneConfig;
use crate::wallet_state::WalletTrackerRow;
use crate::wallet_state::model::{WALLET_TRACKER_CORE_MIN_AGE_MS, WALLET_TRACKER_ORDER_MIN_AGE_MS};

fn terminal(
    tracked: &[&str],
    rows: impl IntoIterator<Item = (&'static str, WalletTrackerRow)>,
) -> TradingTerminal {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.wallet_tracker.tracked_addresses = tracked.iter().map(|s| (*s).to_string()).collect();
    terminal.wallet_tracker.rows = rows
        .into_iter()
        .map(|(address, row)| (address.to_string(), row))
        .collect();
    terminal
}

fn loaded_row(updated_ms: u64) -> WalletTrackerRow {
    WalletTrackerRow {
        snapshot: Some(WalletTrackerSnapshot {
            equity: Some(100.0),
            withdrawable: Some(50.0),
            unrealized_pnl: Some(1.0),
            margin_used_pct: Some(0.1),
            open_trade_count: Some(1),
            open_order_count: 2,
            long_exposure: Some(10.0),
            short_exposure: Some(0.0),
            valuation_warning: None,
        }),
        last_updated_ms: Some(updated_ms),
        orders_last_updated_ms: Some(updated_ms),
        open_order_count: Some(2),
        ..Default::default()
    }
}

#[test]
fn automatic_core_selection_preserves_age_order_ties_and_batch_limits() {
    let now = 120_000;
    let mut terminal = terminal(
        &[
            "fresh", "tie-z", "missing", "tie-a", "retry", "initial", "oldest", "future",
        ],
        [
            ("fresh", loaded_row(now)),
            ("tie-z", loaded_row(now - WALLET_TRACKER_CORE_MIN_AGE_MS)),
            ("tie-a", loaded_row(now - WALLET_TRACKER_CORE_MIN_AGE_MS)),
            (
                "retry",
                WalletTrackerRow {
                    next_core_retry_ms: Some(now + 1),
                    ..loaded_row(0)
                },
            ),
            (
                "initial",
                WalletTrackerRow {
                    last_updated_ms: Some(now + 1),
                    ..Default::default()
                },
            ),
            ("oldest", loaded_row(1)),
            ("future", loaded_row(now + 1)),
        ],
    );

    assert_eq!(
        terminal.wallet_tracker_next_core_addresses(now, 2),
        ["missing", "oldest"]
    );
    assert_eq!(
        terminal.wallet_tracker_next_core_addresses(now, 99),
        ["missing", "oldest", "tie-z", "tie-a", "initial"]
    );
    assert_eq!(
        terminal.wallet_tracker_next_core_addresses(now, 0),
        ["missing"]
    );
    terminal
        .wallet_tracker
        .rows
        .get_mut("retry")
        .expect("retry row")
        .next_core_retry_ms = Some(now);
    assert_eq!(
        terminal.wallet_tracker_next_core_addresses(now, 3),
        ["missing", "retry", "oldest"]
    );
}

#[test]
fn queued_core_selection_preserves_fifo_and_does_not_top_up_from_automatic_work() {
    let now = 120_000;
    let mut terminal = terminal(
        &["missing", "backoff", "queued-b", "queued-a", "automatic"],
        [
            (
                "backoff",
                WalletTrackerRow {
                    next_core_retry_ms: Some(now + 1),
                    ..loaded_row(0)
                },
            ),
            ("queued-b", loaded_row(now)),
            ("queued-a", WalletTrackerRow::default()),
            ("automatic", loaded_row(0)),
        ],
    );
    terminal.wallet_tracker.core_refresh_queue = [
        "unknown", "missing", "backoff", "queued-b", "queued-a", "queued-b",
    ]
    .map(str::to_string)
    .into();

    assert_eq!(
        terminal.wallet_tracker_next_core_addresses(now, 1),
        ["queued-b"]
    );
    assert_eq!(
        terminal.wallet_tracker.core_refresh_queue,
        ["queued-a", "queued-b"]
    );
    assert_eq!(
        terminal.wallet_tracker_next_core_addresses(now, 10),
        ["queued-a", "queued-b"]
    );
    assert!(terminal.wallet_tracker.core_refresh_queue.is_empty());
    assert_eq!(
        terminal.wallet_tracker_next_core_addresses(now, 10),
        ["missing", "queued-a", "automatic"]
    );
}

#[test]
fn automatic_order_selection_preserves_age_order_ties_and_snapshot_requirement() {
    let now = 1_200_000;
    let mut terminal = terminal(
        &[
            "without-snapshot",
            "fresh",
            "tie-z",
            "oldest",
            "tie-a",
            "initial",
            "backoff",
            "future",
        ],
        [
            ("without-snapshot", WalletTrackerRow::default()),
            ("fresh", loaded_row(now)),
            ("tie-z", loaded_row(now - WALLET_TRACKER_ORDER_MIN_AGE_MS)),
            ("oldest", loaded_row(1)),
            ("tie-a", loaded_row(now - WALLET_TRACKER_ORDER_MIN_AGE_MS)),
            (
                "initial",
                WalletTrackerRow {
                    open_order_count: None,
                    ..loaded_row(now + 1)
                },
            ),
            (
                "backoff",
                WalletTrackerRow {
                    next_order_retry_ms: Some(now + 1),
                    ..loaded_row(0)
                },
            ),
            ("future", loaded_row(now + 1)),
        ],
    );

    for expected in ["oldest", "tie-z", "tie-a", "initial"] {
        assert_eq!(
            terminal.wallet_tracker_next_order_address(now).as_deref(),
            Some(expected)
        );
        let row = terminal
            .wallet_tracker
            .rows
            .get_mut(expected)
            .expect("selected row");
        row.orders_last_updated_ms = Some(now);
        row.open_order_count = Some(2);
    }
    assert_eq!(terminal.wallet_tracker_next_order_address(now), None);
    terminal
        .wallet_tracker
        .rows
        .get_mut("backoff")
        .expect("backoff row")
        .next_order_retry_ms = Some(now);
    assert_eq!(
        terminal.wallet_tracker_next_order_address(now).as_deref(),
        Some("backoff")
    );
}

#[test]
fn queued_order_selection_preserves_fifo_without_requiring_a_snapshot() {
    let now = 1_200_000;
    let mut terminal = terminal(
        &["initial", "fresh", "automatic", "backoff", "missing"],
        [
            ("initial", WalletTrackerRow::default()),
            ("fresh", loaded_row(now)),
            ("automatic", loaded_row(0)),
            (
                "backoff",
                WalletTrackerRow {
                    next_order_retry_ms: Some(now + 1),
                    ..loaded_row(0)
                },
            ),
        ],
    );
    terminal.wallet_tracker.order_refresh_queue =
        ["unknown", "missing", "backoff", "initial", "fresh"]
            .map(str::to_string)
            .into();

    assert_eq!(
        terminal.wallet_tracker_next_order_address(now).as_deref(),
        Some("initial")
    );
    assert_eq!(terminal.wallet_tracker.order_refresh_queue, ["fresh"]);
    assert_eq!(
        terminal.wallet_tracker_next_order_address(now).as_deref(),
        Some("fresh")
    );
    assert!(terminal.wallet_tracker.order_refresh_queue.is_empty());
    assert_eq!(
        terminal.wallet_tracker_next_order_address(now).as_deref(),
        Some("automatic")
    );
}

#[test]
fn inflight_rows_block_only_their_refresh_queue_before_draining() {
    for (core_active, order_active) in [(true, false), (false, true), (true, true)] {
        let mut terminal = terminal(
            &["queued"],
            [
                ("queued", loaded_row(0)),
                (
                    "untracked-inflight",
                    WalletTrackerRow {
                        loading: core_active,
                        order_loading: order_active,
                        ..Default::default()
                    },
                ),
            ],
        );
        terminal.wallet_tracker.core_refresh_queue = vec!["queued".to_string()];
        terminal.wallet_tracker.order_refresh_queue = vec!["queued".to_string()];

        let core = terminal.wallet_tracker_next_core_addresses(1_200_000, 2);
        let order = terminal.wallet_tracker_next_order_address(1_200_000);

        assert_eq!(core.is_empty(), core_active);
        assert_eq!(order.is_none(), order_active);
        assert_eq!(
            terminal.wallet_tracker.core_refresh_queue.len(),
            usize::from(core_active)
        );
        assert_eq!(
            terminal.wallet_tracker.order_refresh_queue.len(),
            usize::from(order_active)
        );
    }
}

#[test]
fn queue_admission_and_refresh_all_preserve_tracking_order_and_deduplicate() {
    let mut terminal = terminal(
        &[
            "first",
            "core-loading",
            "order-loading",
            "missing",
            "first",
            "last",
        ],
        [
            ("first", loaded_row(0)),
            (
                "core-loading",
                WalletTrackerRow {
                    loading: true,
                    ..loaded_row(0)
                },
            ),
            (
                "order-loading",
                WalletTrackerRow {
                    order_loading: true,
                    ..loaded_row(0)
                },
            ),
            (
                "last",
                WalletTrackerRow {
                    next_core_retry_ms: Some(u64::MAX),
                    next_order_retry_ms: Some(u64::MAX),
                    ..loaded_row(0)
                },
            ),
        ],
    );
    terminal.wallet_tracker.muted_addresses = vec!["first".to_string()];
    terminal.wallet_tracker.core_refresh_queue = vec!["stale".to_string()];
    terminal.wallet_tracker.order_refresh_queue = vec!["stale".to_string()];
    for address in [
        "not-tracked",
        "first",
        "core-loading",
        "order-loading",
        "missing",
        "last",
        "first",
    ] {
        terminal.queue_wallet_tracker_core_refresh(address.to_string());
        terminal.queue_wallet_tracker_order_refresh(address.to_string());
    }
    assert_eq!(
        terminal.wallet_tracker.core_refresh_queue,
        ["stale", "first", "order-loading", "missing", "last"]
    );
    assert_eq!(
        terminal.wallet_tracker.order_refresh_queue,
        ["stale", "first", "core-loading", "missing", "last"]
    );

    terminal.queue_wallet_tracker_core_refresh_all();

    assert_eq!(
        terminal.wallet_tracker.core_refresh_queue,
        ["first", "order-loading", "missing", "last"]
    );
    assert_eq!(
        terminal.wallet_tracker.order_refresh_queue,
        ["stale", "first", "core-loading", "missing", "last"]
    );
    assert!(!terminal.wallet_tracker.rows.contains_key("missing"));
}

#[test]
fn single_core_refresh_marks_new_and_existing_rows_with_current_context() {
    for existing in [false, true] {
        let mut terminal = terminal(&["selected"], []);
        terminal.read_data_provider_generation = 7;
        terminal.hydromancer_key_generation = 11;
        if existing {
            terminal.wallet_tracker.rows.insert(
                "selected".to_string(),
                WalletTrackerRow {
                    order_loading: true,
                    error: Some("previous error".to_string()),
                    ..loaded_row(42)
                },
            );
        }
        let context = terminal.read_data_request_context();

        let _task = terminal.start_wallet_tracker_core_refresh("selected".to_string());

        let row = terminal
            .wallet_tracker
            .rows
            .get("selected")
            .expect("selected row");
        assert!(row.loading);
        assert_eq!(row.loading_context, Some(context));
        assert_eq!(row.snapshot.is_some(), existing);
        assert_eq!(row.order_loading, existing);
        assert_eq!(row.last_updated_ms, existing.then_some(42));
        assert_eq!(row.error.as_deref(), existing.then_some("previous error"));
    }
}

use super::*;
use futures::StreamExt;
use std::time::Duration;

#[test]
fn ledger_request_uses_inclusive_range_and_actual_account() {
    assert_eq!(
        ledger_request("account", 0, 900),
        json!({
            "type":"userNonFundingLedgerUpdates", "user":"account", "startTime":0, "endTime":900
        })
    );
}

#[test]
fn pagination_overlaps_timestamps_and_detects_stalled_full_pages() {
    assert_eq!(next_cursor(10, Some(20), 500), Ok(Some(20)));
    assert_eq!(next_cursor(10, Some(20), 499), Ok(None));
    assert_eq!(next_cursor(10, Some(20), 1), Ok(None));
    assert_eq!(next_cursor(20, Some(20), 1), Ok(None));
    assert_eq!(next_cursor(20, None, 0), Ok(None));
    assert!(next_cursor(20, Some(20), 500).is_err());
}

fn page(times: impl IntoIterator<Item = u64>, kind: &str) -> Vec<LedgerEntry> {
    times
        .into_iter()
        .map(|time| LedgerEntry {
            time,
            hash: format!("synthetic-{time}"),
            delta: json!({"type": kind, "usdc": "1"}),
        })
        .collect()
}

#[tokio::test]
async fn short_history_finishes_without_a_second_request() {
    for count in [0, 1, LEDGER_PAGE_SIZE - 1] {
        let mut requests = 0;
        let results: Vec<_> = ledger_history("account".into(), 0, 100_000, move |cursor| {
            requests += 1;
            assert_eq!(requests, 1, "short history must need only one request");
            assert_eq!(cursor, 0);
            std::future::ready(Ok(page(1..=count as u64, "deposit")))
        })
        .collect()
        .await;
        assert_eq!(results.len(), 1);
        let snapshot = results[0].as_ref().expect("history");
        assert_eq!(snapshot.entries.len(), count);
        assert!(!snapshot.has_more);
        assert!(snapshot.warning.is_none());
        assert_eq!(snapshot.next_start, Some(40_000));
    }
}

#[tokio::test(start_paused = true)]
async fn rows_arrive_before_a_later_page_times_out() {
    let history = ledger_history("account".into(), 0, 100_000, |cursor| async move {
        if cursor == 0 {
            Ok(page(1..=500, "deposit"))
        } else {
            assert_eq!(cursor, 500);
            tokio::time::sleep(Duration::from_secs(61)).await;
            Ok(Vec::new())
        }
    });
    futures::pin_mut!(history);
    let started = tokio::time::Instant::now();
    let first = history.next().await.expect("first page").expect("rows");
    assert_eq!(started.elapsed(), Duration::ZERO);
    assert_eq!(first.entries.len(), 500);
    assert!(first.has_more);
    assert_eq!(first.next_start, Some(500));
    assert!(history.next().await.expect("timeout result").is_err());
    assert_eq!(started.elapsed(), Duration::from_secs(60));
    assert!(history.next().await.is_none());
}

#[tokio::test]
async fn full_pages_include_ignored_categories_and_overlap_the_boundary() {
    let results: Vec<_> = ledger_history("account".into(), 0, 100_000, |cursor| {
        std::future::ready(Ok(match cursor {
            0 => page(1..=500, "vaultDeposit"),
            500 => page([500, 501], "deposit"),
            _ => panic!("unexpected extra request"),
        }))
    })
    .collect()
    .await;
    assert_eq!(results.len(), 2);
    let first = results[0].as_ref().expect("first page");
    assert!(first.entries.is_empty());
    assert!(first.has_more);
    assert_eq!(first.next_start, Some(500));
    let last = results[1].as_ref().expect("last page");
    assert_eq!(last.entries.len(), 2);
    assert!(!last.has_more);
    assert_eq!(last.next_start, Some(40_000));
}

#[tokio::test]
async fn stalled_full_page_preserves_its_cursor_and_warns() {
    let results: Vec<_> = ledger_history("account".into(), 50, 100_000, |cursor| {
        assert_eq!(cursor, 50);
        std::future::ready(Ok(page([50; LEDGER_PAGE_SIZE], "deposit")))
    })
    .collect()
    .await;
    assert_eq!(results.len(), 1);
    let snapshot = results[0].as_ref().expect("partial history");
    assert_eq!(snapshot.entries.len(), 1, "deduplicate identical records");
    assert!(!snapshot.has_more);
    assert!(snapshot.warning.is_some());
    assert_eq!(snapshot.next_start, Some(50));
}

#[tokio::test]
async fn page_limit_keeps_progress_for_the_next_refresh() {
    let results: Vec<_> = ledger_history("account".into(), 0, 100_000, |cursor| {
        std::future::ready(Ok(page(cursor + 1..=cursor + 500, "vaultDeposit")))
    })
    .collect()
    .await;
    assert_eq!(results.len(), MAX_PAGES);
    let snapshot = results
        .last()
        .expect("final page")
        .as_ref()
        .expect("history");
    assert!(!snapshot.has_more);
    assert!(snapshot.warning.is_some());
    assert_eq!(snapshot.next_start, Some(50_000));
}

#[test]
fn native_bridges_preserve_amounts_without_inventing_wallets_or_finality() {
    for (kind, amount) in [("deposit", json!("100.123456")), ("withdraw", json!(100))] {
        let entry = bridge_entry(
            LedgerEntry {
                time: 100,
                hash: "ledger".into(),
                delta: json!({"type":kind, "usdc":amount, "fee":"1"}),
            },
            "account",
        )
        .expect("valid bridge")
        .expect("bridge row");
        assert_eq!(entry.asset, "USDC");
        assert_eq!(entry.fee.as_deref(), Some("1"));
        assert!(entry.source_tx.is_none());
        assert!(entry.destination_tx.is_none());
        if kind == "deposit" {
            assert_eq!(entry.amount, "100.123456");
            assert!(entry.source_address.is_none());
            assert_eq!(entry.destination_address.as_deref(), Some("account"));
        } else {
            assert_eq!(entry.status, "Debited");
            assert!(entry.destination_address.is_none());
        }
    }
}

#[test]
fn internal_transfers_are_not_external_deposits() {
    for kind in [
        "spotTransfer",
        "internalTransfer",
        "accountClassTransfer",
        "vaultDeposit",
        "subAccountTransfer",
    ] {
        assert!(
            bridge_entry(
                LedgerEntry {
                    time: 1,
                    hash: "test".into(),
                    delta: json!({"type":kind})
                },
                "account"
            )
            .expect("ignored event")
            .is_none()
        );
    }
}

#[test]
fn invalid_bridge_amount_is_an_error_instead_of_zero() {
    assert!(
        bridge_entry(
            LedgerEntry {
                time: 1,
                hash: "test".into(),
                delta: json!({"type":"deposit", "usdc":"NaN"})
            },
            "account"
        )
        .is_err()
    );
}

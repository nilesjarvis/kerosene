use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

fn candle(volume: f64, close: f64) -> Candle {
    Candle::test_ohlcv(0, 0, [0.0, 0.0, 0.0, close], volume)
}

#[test]
fn outcome_volume_from_candles_sums_positive_finite_contract_and_notional_volume() {
    let candles = vec![
        candle(10.0, 0.25),
        candle(f64::NAN, 0.25),
        candle(-4.0, 0.25),
        candle(5.5, 0.50),
        candle(f64::INFINITY, 0.25),
        candle(1.0, f64::NAN),
    ];

    assert_eq!(
        outcome_volume_from_candles(&candles),
        OutcomeVolume24h {
            contract: 16.5,
            notional: 5.25,
        }
    );
}

struct ActiveRead<'a>(&'a AtomicUsize);

impl Drop for ActiveRead<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn outcome_volume_fetches_never_exceed_two_concurrent_reads() {
    let active = AtomicUsize::new(0);
    let peak = AtomicUsize::new(0);
    let result = fetch_outcome_volumes_with((0..20).map(|n| format!("#{n}")).collect(), |symbol| {
        let active = &active;
        let peak = &peak;
        async move {
            let count = active.fetch_add(1, Ordering::SeqCst) + 1;
            let _read = ActiveRead(active);
            peak.fetch_max(count, Ordering::SeqCst);
            tokio::task::yield_now().await;
            Ok((symbol, OutcomeVolume24h::default()))
        }
    })
    .await
    .expect("volumes");

    assert_eq!(result.len(), 20);
    assert_eq!(peak.load(Ordering::SeqCst), 2);
    assert_eq!(active.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn cancelling_outcome_volume_batch_drops_reads_without_starting_queued_symbols() {
    let started = AtomicUsize::new(0);
    let active = AtomicUsize::new(0);
    let mut batch = Box::pin(fetch_outcome_volumes_with(
        (0..20).map(|n| format!("#{n}")).collect(),
        |symbol| {
            let started = &started;
            let active = &active;
            async move {
                started.fetch_add(1, Ordering::SeqCst);
                active.fetch_add(1, Ordering::SeqCst);
                let _read = ActiveRead(active);
                futures::future::pending::<()>().await;
                Ok((symbol, OutcomeVolume24h::default()))
            }
        },
    ));
    assert!(futures::poll!(batch.as_mut()).is_pending());
    assert_eq!(started.load(Ordering::SeqCst), 2);
    assert_eq!(active.load(Ordering::SeqCst), 2);

    drop(batch);

    assert_eq!(active.load(Ordering::SeqCst), 0);
    assert_eq!(started.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn bounded_outcome_volume_fetch_keeps_successes_when_another_symbol_fails() {
    let volumes = fetch_outcome_volumes_with(vec!["#1".into(), "#2".into()], |symbol| async move {
        if symbol == "#1" {
            Err("unavailable".into())
        } else {
            Ok((
                symbol,
                OutcomeVolume24h {
                    contract: 2.0,
                    notional: 1.0,
                },
            ))
        }
    })
    .await
    .expect("partial success");
    assert_eq!(volumes.len(), 1);
    assert_eq!(volumes["#2"].contract, 2.0);

    let failed =
        fetch_outcome_volumes_with(vec!["#1".into()], |_| async { Err("unavailable".into()) })
            .await;
    assert_eq!(failed, Err("unavailable".into()));
}

#[tokio::test]
async fn empty_outcome_volume_batch_does_not_fetch() {
    let volumes = fetch_outcome_volumes_with(Vec::new(), |_| async {
        panic!("empty batch must not fetch")
    })
    .await
    .expect("empty batch");
    assert!(volumes.is_empty());
}

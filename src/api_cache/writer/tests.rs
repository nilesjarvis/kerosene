use super::super::storage::load_json;
use super::super::tests::test_cache_dir;
use super::*;
use std::fs;

fn batch_job(code: usize) -> CacheWrite {
    let root = PathBuf::from("cache-root");
    let symbol = ["BTC", "ETH"][code % 2];
    let path = json_path(
        &root,
        CANDLE_CACHE_NAMESPACE,
        &candle_key(ChartBackfillSource::Hyperliquid, symbol, "1m"),
    );
    match code / 2 {
        0 => CacheWrite::SaveCandles {
            root,
            source: ChartBackfillSource::Hyperliquid,
            symbol: symbol.into(),
            interval: "1m".into(),
            candles: Vec::new(),
            observed_at_ms: 1,
        },
        1 => CacheWrite::SaveBytes {
            path,
            bytes: vec![1],
        },
        2 => CacheWrite::MergeCandles {
            root,
            source: ChartBackfillSource::Hyperliquid,
            symbol: symbol.into(),
            interval: "1m".into(),
            candles: Vec::new(),
            observed_at_ms: 1,
        },
        3 => CacheWrite::Remove { path },
        _ => panic!("test job code must be in 0..8"),
    }
}

#[test]
fn coalescing_matches_the_later_superseder_rule_for_all_short_batches() {
    // Four job kinds and two targets cover both save representations, merges,
    // removals, and interleaved paths. Check every sequence up to five jobs.
    for length in 0..=5 {
        for encoded in 0..8usize.pow(length) {
            let mut remaining = encoded;
            let codes: Vec<_> = (0..length)
                .map(|_| {
                    let code = remaining % 8;
                    remaining /= 8;
                    code
                })
                .collect();
            let expected: Vec<_> = codes
                .iter()
                .enumerate()
                .map(|(index, code)| {
                    let is_save = code / 2 < 2;
                    !is_save
                        || !codes[index + 1..].iter().any(|later| {
                            let same_target = later % 2 == code % 2;
                            let supersedes = later / 2 != 2; // A merge never supersedes a save.
                            same_target && supersedes
                        })
                })
                .collect();
            let batch: Vec<_> = codes.iter().copied().map(batch_job).collect();
            assert_eq!(writes_to_run(&batch), expected, "batch {codes:?}");
        }
    }
}

fn candle_job(root: &std::path::Path, timestamp: u64, merge: bool) -> CacheWrite {
    let candles = vec![Candle::test_flat(timestamp, 100.0)];
    if merge {
        CacheWrite::MergeCandles {
            root: root.to_path_buf(),
            source: ChartBackfillSource::Hyperliquid,
            symbol: "BTC".into(),
            interval: "1m".into(),
            candles,
            observed_at_ms: 300_000,
        }
    } else {
        CacheWrite::SaveCandles {
            root: root.to_path_buf(),
            source: ChartBackfillSource::Hyperliquid,
            symbol: "BTC".into(),
            interval: "1m".into(),
            candles,
            observed_at_ms: 300_000,
        }
    }
}

fn cached_timestamps(root: &std::path::Path) -> Vec<u64> {
    load_json::<Vec<Candle>>(
        root,
        CANDLE_CACHE_NAMESPACE,
        &candle_key(ChartBackfillSource::Hyperliquid, "BTC", "1m"),
    )
    .expect("read cached candles")
    .expect("candle snapshot should exist")
    .payload
    .into_iter()
    .map(|candle| candle.open_time)
    .collect()
}

#[test]
fn write_batch_keeps_save_merge_order_and_continues_after_failed_writes() {
    let root = test_cache_dir("batch-order");
    fs::create_dir_all(&root).expect("create test cache directory");
    let blocker = root.join("file-blocking-directory");
    fs::write(&blocker, b"blocker").expect("create blocking file");
    let other = root.join("other.json");

    run_write_batch(vec![
        candle_job(&root, 60_000, false),
        CacheWrite::SaveBytes {
            path: blocker.join("child.json"),
            bytes: vec![1],
        },
        candle_job(&root, 120_000, true),
        CacheWrite::SaveBytes {
            path: other.clone(),
            bytes: vec![2],
        },
    ]);

    assert_eq!(cached_timestamps(&root), vec![60_000, 120_000]);
    assert_eq!(fs::read(&other).expect("later write must run"), vec![2]);
    assert_eq!(
        fs::read(&blocker).expect("blocking file remains"),
        b"blocker"
    );
    fs::remove_dir_all(root).expect("remove test cache");
}

#[test]
fn write_batch_removes_old_candles_before_a_later_merge() {
    let root = test_cache_dir("remove-before-merge");
    let first = candle_job(&root, 60_000, false);
    let path = first.target();
    first.run();
    assert_eq!(cached_timestamps(&root), vec![60_000]);

    run_write_batch(vec![
        CacheWrite::Remove { path },
        candle_job(&root, 120_000, true),
    ]);

    assert_eq!(cached_timestamps(&root), vec![120_000]);
    fs::remove_dir_all(root).expect("remove test cache");
}

#[test]
fn coalesces_saves_superseded_by_a_later_save_to_the_same_path() {
    let path_a = PathBuf::from("/tmp/cache/a.json");
    let path_b = PathBuf::from("/tmp/cache/b.json");
    let batch = vec![
        CacheWrite::SaveBytes {
            path: path_a.clone(),
            bytes: vec![1],
        },
        CacheWrite::SaveBytes {
            path: path_b,
            bytes: vec![2],
        },
        CacheWrite::SaveBytes {
            path: path_a,
            bytes: vec![3],
        },
    ];
    // The first save to path_a is redundant; the later one wins.
    assert_eq!(writes_to_run(&batch), vec![false, true, true]);
}

#[test]
fn remove_supersedes_earlier_save_but_merge_does_not() {
    let path = PathBuf::from("/tmp/cache/candles.json");
    let removal = vec![
        CacheWrite::SaveBytes {
            path: path.clone(),
            bytes: vec![1],
        },
        CacheWrite::Remove { path },
    ];
    assert_eq!(writes_to_run(&removal), vec![false, true]);

    // A merge reads the saved file, so an earlier save to the same key must
    // still run even though it shares the target path.
    let merge = vec![
        CacheWrite::SaveCandles {
            root: PathBuf::from("/tmp/cache"),
            source: ChartBackfillSource::Hyperliquid,
            symbol: "BTC".to_string(),
            interval: "1m".to_string(),
            candles: Vec::new(),
            observed_at_ms: 1,
        },
        CacheWrite::MergeCandles {
            root: PathBuf::from("/tmp/cache"),
            source: ChartBackfillSource::Hyperliquid,
            symbol: "BTC".to_string(),
            interval: "1m".to_string(),
            candles: Vec::new(),
            observed_at_ms: 1,
        },
    ];
    assert_eq!(writes_to_run(&merge), vec![true, true]);
}

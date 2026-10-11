use super::*;
use crate::account_analytics::PortfolioBucket;
use crate::api_cache::{storage::write_bytes_atomic, tests::test_cache_dir};
use std::fs;

const ADDRESS: &str = "0xabcdef0000000000000000000000000000000000";
const OTHER_ADDRESS: &str = "0x1111111111111111111111111111111111111111";

fn history() -> PortfolioHistory {
    PortfolioHistory {
        buckets: [
            "day",
            "week",
            "month",
            "allTime",
            "perpDay",
            "perpWeek",
            "perpMonth",
            "perpAllTime",
        ]
        .into_iter()
        .map(|key| {
            (
                key.to_string(),
                PortfolioBucket {
                    account_value_history: vec![(1_000, 100.0), (2_000, 120.0)],
                    pnl_history: vec![(1_000, 10.0), (2_000, 30.0)],
                    vlm: Some(40.0),
                    skipped_invalid_points: 1,
                    invalid_vlm: false,
                },
            )
        })
        .collect(),
    }
}

#[test]
fn combined_portfolio_disk_snapshot_restores_all_buckets_and_original_fetch_time() {
    let root = test_cache_dir("combined-portfolio-restart");
    let history = history();
    let (path, bytes) = snapshot_bytes(&root, ADDRESS, &history, 3_000).expect("encode snapshot");
    write_bytes_atomic(&path, &bytes).expect("save snapshot");

    // Read from disk with no retained runtime state, using another address case.
    let mixed_case = ADDRESS.to_uppercase().replacen("0X", "0x", 1);
    let cached = load_from_dir(&root, &mixed_case)
        .expect("read snapshot")
        .expect("cache hit");
    assert_eq!(cached.fetched_at_ms, 3_000);
    assert_eq!(
        serde_json::to_value(cached.history).expect("loaded history"),
        serde_json::to_value(history).expect("original history"),
    );
    assert!(
        load_from_dir(&root, OTHER_ADDRESS)
            .expect("other wallet")
            .is_none()
    );
    assert!(!path.to_string_lossy().contains(ADDRESS));
    assert!(
        !String::from_utf8(bytes)
            .expect("json bytes")
            .contains(ADDRESS)
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).expect("metadata").permissions().mode() & 0o777,
            0o600
        );
    }
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn combined_portfolio_disk_sync_replaces_revised_and_expired_samples() {
    let root = test_cache_dir("combined-portfolio-replace");
    let mut history = history();
    let (path, bytes) = snapshot_bytes(&root, ADDRESS, &history, 3_000).expect("old snapshot");
    write_bytes_atomic(&path, &bytes).expect("save old snapshot");
    history.buckets.get_mut("day").expect("day").pnl_history = vec![(2_000, 35.0), (4_000, 50.0)];
    history
        .buckets
        .get_mut("allTime")
        .expect("all time")
        .pnl_history[0]
        .1 = 12.0;
    let (_, bytes) = snapshot_bytes(&root, ADDRESS, &history, 5_000).expect("new snapshot");
    write_bytes_atomic(&path, &bytes).expect("save new snapshot");
    let cached = load_from_dir(&root, ADDRESS).expect("read").expect("hit");
    assert_eq!(cached.fetched_at_ms, 5_000);
    assert_eq!(
        cached.history.buckets["day"].pnl_history,
        vec![(2_000, 35.0), (4_000, 50.0)]
    );
    assert_eq!(
        cached.history.buckets["allTime"].pnl_history[0],
        (1_000, 12.0)
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn combined_portfolio_cache_rejects_corrupt_incompatible_and_future_snapshots() {
    let root = test_cache_dir("combined-portfolio-invalid");
    assert!(
        load_from_dir(&root, ADDRESS)
            .expect("missing cache")
            .is_none()
    );
    let (path, bytes) = snapshot_bytes(&root, ADDRESS, &history(), 3_000).expect("snapshot");
    write_bytes_atomic(&path, b"{broken").expect("write corrupt cache");
    assert!(load_from_dir(&root, ADDRESS).is_err());
    let original: serde_json::Value = serde_json::from_slice(&bytes).expect("envelope");
    for (field, value) in [
        ("schema_version", serde_json::json!(999)),
        ("key", serde_json::json!(["wrong-wallet"])),
        ("fetched_at_ms", serde_json::json!(u64::MAX)),
        ("fetched_at_ms", serde_json::json!(0)),
        ("payload", serde_json::json!({"buckets": {}})),
    ] {
        let mut envelope = original.clone();
        envelope[field] = value;
        write_bytes_atomic(&path, &serde_json::to_vec(&envelope).expect("json")).expect("write");
        assert!(
            load_from_dir(&root, ADDRESS).expect("load").is_none(),
            "{field}"
        );
    }
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn combined_portfolio_cache_rejects_invalid_data_and_redacts_debug_output() {
    assert!(wallet_key("../../invalid").is_err());
    let cached = CachedPortfolioHistory {
        history: history(),
        fetched_at_ms: 3_000,
    };
    let debug = format!("{cached:?}");
    assert!(!debug.contains("pnl_history"));
    assert!(!debug.contains("100.0"));
    let mut invalid = history();
    invalid.buckets.get_mut("day").expect("day").pnl_history[0].1 = f64::NAN;
    assert!(!valid_history(&invalid));
    assert!(snapshot_bytes(Path::new("unused"), ADDRESS, &invalid, 3_000).is_err());
    invalid = history();
    invalid
        .buckets
        .get_mut("day")
        .expect("day")
        .account_value_history[0]
        .0 = 0;
    assert!(!valid_history(&invalid));
}

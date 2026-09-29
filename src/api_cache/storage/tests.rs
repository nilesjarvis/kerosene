use super::super::tests::test_cache_dir;
use super::*;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn cache_envelope_round_trips_and_rejects_mismatched_metadata() {
    let root = test_cache_dir("envelope");
    let key = vec!["BTC".to_string(), "1m".to_string()];
    let payload = vec![1_u64, 2, 3];
    save_json(&root, "test", &key, 100, Some(90), &payload).expect("save envelope");
    let cached = load_json::<Vec<u64>>(&root, "test", &key)
        .expect("load envelope")
        .expect("cache hit");
    assert_eq!(cached.fetched_at_ms, 100);
    assert_eq!(cached.complete_through_ms, Some(90));
    assert_eq!(cached.payload, payload);

    let path = json_path(&root, "test", &key);
    let original: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("read envelope bytes"))
            .expect("decode envelope");
    for (field, value) in [
        (
            "schema_version",
            serde_json::json!(CACHE_SCHEMA_VERSION + 1),
        ),
        ("namespace", serde_json::json!("other")),
        ("key", serde_json::json!(["ETH", "1m"])),
    ] {
        let mut changed = original.clone();
        changed[field] = value;
        fs::write(
            &path,
            serde_json::to_vec(&changed).expect("encode envelope"),
        )
        .expect("replace envelope");
        assert!(
            load_json::<Vec<u64>>(&root, "test", &key)
                .expect("read cache")
                .is_none()
        );
    }
    fs::remove_dir_all(root).expect("remove test cache");
}

#[test]
fn atomic_write_replaces_contents_and_cleans_up_failed_replacements() {
    let root = test_cache_dir("atomic-replacement");
    let path = root.join("cache.json");
    write_bytes_atomic(&path, b"before").expect("initial write");
    write_bytes_atomic(&path, b"after").expect("replace cached file");
    assert_eq!(fs::read(&path).expect("read cached file"), b"after");
    #[cfg(unix)]
    assert_eq!(
        fs::metadata(&path)
            .expect("file metadata")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );

    let directory = root.join("existing-directory");
    fs::create_dir(&directory).expect("create directory blocking replacement");
    assert!(write_bytes_atomic(&directory, b"cannot replace a directory").is_err());
    assert!(directory.is_dir());
    assert_eq!(
        fs::read_dir(&root).expect("list cache directory").count(),
        2,
        "failed replacement must not leave a temporary file"
    );
    fs::remove_dir_all(root).expect("remove test cache");
}

#[test]
fn cache_component_escapes_path_separators_and_market_prefixes() {
    assert_eq!(cache_component("BTC"), "BTC");
    assert_eq!(cache_component("xyz:NVDA"), "xyz_3aNVDA");
    assert_eq!(cache_component("@107"), "_40107");
    assert_eq!(cache_component("../secret"), "_2e_2e_2fsecret");
}

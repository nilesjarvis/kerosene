use super::{clear_cache_path_family, replace_cache_file, unique_temp_cache_path};
use std::collections::HashSet;
use std::path::Path;

#[test]
fn journal_cache_temp_paths_are_unique_and_adjacent() {
    let target = Path::new("/tmp/kerosene-test/journal_cache_0xabc.json");
    let mut seen = HashSet::new();
    for _ in 0..64 {
        let temp = unique_temp_cache_path(target);
        assert_eq!(temp.parent(), target.parent());
        assert!(
            temp.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("journal_cache_0xabc.json.tmp."))
        );
        assert!(seen.insert(temp));
    }
}

#[test]
fn clear_cache_path_family_removes_cache_and_temp_files() {
    let dir = std::env::temp_dir().join(format!(
        "kerosene-journal-cache-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp test directory");
    let target = dir.join("journal_cache_0xabc.json");
    let temp_a = dir.join("journal_cache_0xabc.json.tmp.1");
    let temp_b = dir.join("journal_cache_0xabc.json.tmp.2");
    let unrelated = dir.join("journal_cache_0xdef.json");
    std::fs::write(&target, "[]").expect("write cache");
    std::fs::write(&temp_a, "[]").expect("write temp cache");
    std::fs::write(&temp_b, "[]").expect("write temp cache");
    std::fs::write(&unrelated, "[]").expect("write unrelated cache");

    let removed = clear_cache_path_family(&target).expect("clear cache path family");

    assert_eq!(removed, 3);
    assert!(!target.exists());
    assert!(!temp_a.exists());
    assert!(!temp_b.exists());
    assert!(unrelated.exists());

    let _ = std::fs::remove_file(unrelated);
    let _ = std::fs::remove_dir(dir);
}

#[test]
fn clear_cache_path_family_tolerates_missing_cache() {
    let dir = std::env::temp_dir().join(format!(
        "kerosene-journal-cache-missing-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp test directory");
    let target = dir.join("journal_cache_0xabc.json");

    let removed = clear_cache_path_family(&target).expect("clear missing cache path family");

    assert_eq!(removed, 0);
    let _ = std::fs::remove_dir(dir);
}

#[test]
fn clear_cache_path_family_errors_redact_cache_path() {
    let dir = std::env::temp_dir().join(format!(
        "kerosene-journal-cache-error-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp test directory");
    let target = dir.join("journal_cache_0xabc0000000000000000000000000000000000000.json");
    std::fs::create_dir_all(&target).expect("create directory where cache file should be");

    let error = clear_cache_path_family(&target).expect_err("directory cache path should fail");

    assert!(error.contains("<config-dir>/journal_cache_<redacted>.json"));
    assert!(!error.contains(&dir.display().to_string()));
    assert!(!error.contains("0xabc0000000000000000000000000000000000000"));

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn replacing_an_existing_cache_works_on_the_host_platform() {
    let dir = std::env::temp_dir().join(format!(
        "kerosene-journal-cache-replace-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp test directory");
    let target = dir.join("journal_cache_0xabc.json");
    let temp = dir.join("journal_cache_0xabc.json.tmp.test");
    std::fs::write(&target, "old").expect("write old cache");
    std::fs::write(&temp, "new").expect("write new cache");

    replace_cache_file(&temp, &target).expect("replace existing cache");

    assert_eq!(std::fs::read_to_string(&target).unwrap_or_default(), "new");
    assert!(!temp.exists());
    let _ = std::fs::remove_file(target);
    let _ = std::fs::remove_dir(dir);
}

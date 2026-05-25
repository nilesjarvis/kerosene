use super::unique_temp_cache_path;
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

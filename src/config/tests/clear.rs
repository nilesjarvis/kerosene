use super::super::clear::clear_config_path_family;
use super::unique_test_config_path;

#[test]
fn clear_config_path_family_removes_primary_backup_and_temps() {
    let path = unique_test_config_path("clear-family");
    let parent = path.parent().expect("test path should have parent");
    let backup_path = super::super::files::backup_config_path(&path);
    let temp_path = path.with_extension("json.tmp-test");
    let unrelated_path = parent.join("other.json.tmp-test");

    std::fs::create_dir_all(parent).expect("test directory can be created");
    std::fs::write(&path, "{}").expect("primary write succeeds");
    std::fs::write(&backup_path, "{}").expect("backup write succeeds");
    std::fs::write(&temp_path, "{}").expect("temp write succeeds");
    std::fs::write(&unrelated_path, "{}").expect("unrelated write succeeds");

    let removed = clear_config_path_family(&path).expect("clear should succeed");

    assert_eq!(removed, 3);
    assert!(!path.exists());
    assert!(!backup_path.exists());
    assert!(!temp_path.exists());
    assert!(unrelated_path.exists());

    let _ = std::fs::remove_dir_all(parent);
}

mod clear;
mod files;
mod serialization;

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

fn config_warning_guard() -> MutexGuard<'static, ()> {
    static WARNING_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let guard = match WARNING_TEST_LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let _ = super::take_config_warnings();
    guard
}

#[test]
fn config_warnings_are_isolated_by_test_thread() {
    let _warning_guard = config_warning_guard();

    super::push_config_warning("main warning".to_string());
    let worker_warnings = std::thread::spawn(|| {
        super::push_config_warning("worker warning".to_string());
        super::take_config_warnings()
    })
    .join()
    .expect("config warning worker should finish");

    assert_eq!(worker_warnings, vec!["worker warning".to_string()]);
    assert_eq!(
        super::take_config_warnings(),
        vec!["main warning".to_string()]
    );
    assert!(super::take_config_warnings().is_empty());
}

fn unique_test_config_path(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir()
        .join(format!("kerosene-config-test-{name}-{nanos}"))
        .join("config.json")
}

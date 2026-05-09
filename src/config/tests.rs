mod clear;
mod files;
mod serialization;

use std::path::PathBuf;

fn unique_test_config_path(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir()
        .join(format!("kerosene-config-test-{name}-{nanos}"))
        .join("config.json")
}

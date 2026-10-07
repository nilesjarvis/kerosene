use super::*;
use crate::agent_snapshot::{activate_agent_snapshot, write_agent_snapshot};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "kerosene-snapshot-cleanup-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock")
                .as_nanos(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&root).expect("test root");
        Self(root)
    }

    fn snapshot(&self, name: &str) -> PathBuf {
        let workspace = self.0.join(name);
        let staged = futures::executor::block_on(write_agent_snapshot(
            workspace.clone(),
            1,
            2,
            br#"{"account":{"positions":[{"coin":"BTC","size":1}]}}"#.to_vec(),
        ))
        .expect("write real snapshot path");
        activate_agent_snapshot(&workspace, &staged).expect("activate snapshot");
        workspace
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn startup_removes_crash_residue_including_all_staged_snapshots() {
    let root = TestRoot::new();
    let workspace = root.snapshot("kerosene-agent-123");
    for (generation, request_id) in [(0, 0), (4, 5), (u64::MAX, u64::MAX)] {
        futures::executor::block_on(write_agent_snapshot(
            workspace.clone(),
            generation,
            request_id,
            b"{}".to_vec(),
        ))
        .expect("staged crash residue");
    }
    fs::write(workspace.join("kerosene-extension.ts"), "extension").expect("extension");
    fs::create_dir(workspace.join("pi-config")).expect("Pi config");
    fs::write(workspace.join("pi-config/models.json"), "{}").expect("models");

    cleanup_stale_snapshots_in(&root.0, 456, |_| false);

    let mut remaining = fs::read_dir(&workspace)
        .expect("workspace preserved")
        .map(|entry| entry.expect("entry").file_name())
        .collect::<Vec<_>>();
    remaining.sort();
    assert_eq!(remaining, ["kerosene-extension.ts", "pi-config"]);
    assert!(workspace.join("pi-config/models.json").is_file());
    cleanup_stale_snapshots_in(&root.0, 456, |_| false);
    cleanup_stale_snapshots_in(&root.0.join("absent"), 456, |_| false);
}

#[test]
fn startup_preserves_live_process_snapshots_but_clears_its_reused_pid() {
    let root = TestRoot::new();
    let live = root.snapshot("kerosene-agent-123");
    let reused = root.snapshot("kerosene-agent-456");

    cleanup_stale_snapshots_in(&root.0, 456, |_| true);

    assert!(live.join("snapshot.json").is_file());
    assert!(!reused.join("snapshot.json").exists());
}

#[test]
fn startup_preserves_unrecognized_paths_and_nested_content() {
    let root = TestRoot::new();
    for name in [
        "kerosene-agent-0",
        "kerosene-agent-00123",
        "kerosene-agent-+123",
        "kerosene-agent-123-backup",
        "kerosene-agent-sessions-test",
        "unrelated",
    ] {
        let workspace = root.snapshot(name);
        cleanup_stale_snapshots_in(&root.0, 456, |_| false);
        assert!(workspace.join("snapshot.json").is_file());
    }
    let workspace = root.snapshot("kerosene-agent-123");
    for name in [
        "snapshot-backup.json",
        "snapshot-1-2.json.bak",
        "notes.json",
    ] {
        fs::write(workspace.join(name), "keep").expect("unrelated file");
    }
    fs::create_dir(workspace.join("snapshot-1-2.json")).expect("nested directory");
    fs::write(workspace.join("snapshot-1-2.json/keep"), "keep").expect("nested file");

    cleanup_stale_snapshots_in(&root.0, 456, |_| false);

    assert!(!workspace.join("snapshot.json").exists());
    assert_eq!(fs::read_dir(workspace).expect("workspace").count(), 4);
}

#[cfg(unix)]
#[test]
fn startup_does_not_follow_symlinks_or_scan_shared_writable_directories() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = TestRoot::new();
    let target = root.snapshot("unrelated");
    symlink(&target, root.0.join("kerosene-agent-123")).expect("directory symlink");
    let workspace = root.snapshot("kerosene-agent-124");
    symlink(
        target.join("snapshot.json"),
        workspace.join("snapshot-1-2.json"),
    )
    .expect("snapshot symlink");
    let shared = root.snapshot("kerosene-agent-125");
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o777)).expect("shared mode");

    cleanup_stale_snapshots_in(&root.0, 456, |_| false);

    assert!(target.join("snapshot.json").is_file());
    assert!(workspace.join("snapshot-1-2.json").is_symlink());
    assert!(shared.join("snapshot.json").is_file());
    assert!(!workspace.join("snapshot.json").exists());
}

#[test]
fn process_probe_preserves_running_processes_and_recognizes_exited_processes() {
    assert!(process_is_running(std::process::id()));
    let mut child = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .arg("--list")
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("short-lived child");
    let pid = child.id();
    child.wait().expect("child reaped");
    assert!(!process_is_running(pid));
}

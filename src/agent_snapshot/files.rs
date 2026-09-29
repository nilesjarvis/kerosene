use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Assistant Snapshot Files
// ---------------------------------------------------------------------------

pub(crate) async fn write_agent_snapshot(
    workspace_dir: PathBuf,
    generation: u64,
    request_id: u64,
    snapshot: Vec<u8>,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(&workspace_dir)
        .map_err(|error| format!("Could not create the assistant workspace: {error}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&workspace_dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("Could not secure the assistant workspace: {error}"))?;
    }

    let path = staged_snapshot_path(&workspace_dir, generation, request_id);
    let mut options = OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&path)
        .map_err(|error| format!("Could not open the assistant snapshot: {error}"))?;
    file.write_all(&snapshot)
        .map_err(|error| format!("Could not write the assistant snapshot: {error}"))?;
    Ok(path)
}

pub(crate) fn activate_agent_snapshot(
    workspace_dir: &Path,
    staged_path: &Path,
) -> Result<PathBuf, String> {
    let active_path = workspace_dir.join("snapshot.json");
    match std::fs::rename(staged_path, &active_path) {
        Ok(()) => Ok(active_path),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            std::fs::remove_file(&active_path)
                .map_err(|error| format!("Could not replace the assistant snapshot: {error}"))?;
            std::fs::rename(staged_path, &active_path)
                .map_err(|error| format!("Could not activate the assistant snapshot: {error}"))?;
            Ok(active_path)
        }
        Err(error) => Err(format!(
            "Could not activate the assistant snapshot: {error}"
        )),
    }
}

pub(crate) fn workspace_dir() -> PathBuf {
    std::env::temp_dir().join(format!("kerosene-agent-{}", std::process::id()))
}

pub(crate) fn clear_sensitive_runtime_files(
    workspace_dir: &Path,
    generation: u64,
    request_id: u64,
) {
    let snapshot_path = workspace_dir.join("snapshot.json");
    let _ = std::fs::remove_file(snapshot_path);
    let _ = std::fs::remove_file(staged_snapshot_path(workspace_dir, generation, request_id));
}

fn staged_snapshot_path(workspace_dir: &Path, generation: u64, request_id: u64) -> PathBuf {
    workspace_dir.join(format!("snapshot-{generation}-{request_id}.json"))
}

use std::fs;
use std::path::Path;

/// Run once, before any assistant tasks start. A directory bearing our new PID
/// can contain residue from a previous process that used the same PID.
pub(crate) fn cleanup_stale_runtime_snapshots() {
    cleanup_stale_snapshots_in(
        &std::env::temp_dir(),
        std::process::id(),
        process_is_running,
    );
}

fn cleanup_stale_snapshots_in(temp_dir: &Path, startup_pid: u32, is_running: impl Fn(u32) -> bool) {
    let Ok(entries) = fs::read_dir(temp_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(workspace_pid) else {
            continue;
        };
        if pid != startup_pid && is_running(pid) {
            continue;
        }
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !owned_directory(&metadata) {
            continue;
        }

        // Anchor Unix deletions to the inspected directory, even if its path
        // is replaced while scanning the shared temporary directory.
        #[cfg(unix)]
        let directory = {
            use std::os::unix::fs::OpenOptionsExt;
            let Ok(directory) = fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
                .open(&path)
            else {
                continue;
            };
            if !directory
                .metadata()
                .is_ok_and(|meta| owned_directory(&meta))
            {
                continue;
            }
            directory
        };

        let Ok(files) = fs::read_dir(&path) else {
            continue;
        };
        for file in files.flatten() {
            if !file.file_name().to_str().is_some_and(is_snapshot_name)
                || !file.file_type().is_ok_and(|kind| kind.is_file())
            {
                continue;
            }
            #[cfg(unix)]
            {
                use std::os::fd::AsRawFd;
                use std::os::unix::ffi::OsStrExt;
                if let Ok(name) = std::ffi::CString::new(file.file_name().as_bytes()) {
                    // SAFETY: the descriptor is owned by `directory`; name is
                    // a single NUL-terminated filename, and flags forbid rmdir.
                    unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) };
                }
            }
            #[cfg(not(unix))]
            let _ = fs::remove_file(file.path());
        }
        // Keep extension/config files and any unrecognized content. Never
        // recursively delete a directory based only on a filename prefix.
    }
}

fn workspace_pid(name: &str) -> Option<u32> {
    let suffix = name.strip_prefix("kerosene-agent-")?;
    let pid = suffix.parse::<u32>().ok()?;
    (pid > 0 && suffix == pid.to_string()).then_some(pid)
}

fn is_snapshot_name(name: &str) -> bool {
    if name == "snapshot.json" {
        return true;
    }
    let Some((generation, request_id)) = name
        .strip_prefix("snapshot-")
        .and_then(|name| name.strip_suffix(".json"))
        .and_then(|name| name.split_once('-'))
    else {
        return false;
    };
    [generation, request_id].into_iter().all(|part| {
        !part.is_empty()
            && part.bytes().all(|byte| byte.is_ascii_digit())
            && part.parse::<u64>().is_ok()
    })
}

fn owned_directory(metadata: &fs::Metadata) -> bool {
    if !metadata.file_type().is_dir() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // SAFETY: geteuid takes no arguments and has no preconditions.
        metadata.uid() == unsafe { libc::geteuid() } && metadata.mode() & 0o022 == 0
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Windows temp_dir is per-user. Do not traverse junctions/reparse points.
        metadata.file_attributes() & 0x400 == 0
    }
    #[cfg(not(any(unix, windows)))]
    false
}

#[cfg(unix)]
fn process_is_running(pid: u32) -> bool {
    let Some(pid) = libc::pid_t::try_from(pid).ok().filter(|pid| *pid > 0) else {
        return true;
    };
    // SAFETY: a positive PID with signal zero probes existence without sending
    // a signal. Permission errors and all other unknown states must be kept.
    let exists = unsafe { libc::kill(pid, 0) == 0 };
    exists || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
}

#[cfg(windows)]
fn process_is_running(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_INVALID_PARAMETER, GetLastError, WAIT_OBJECT_0,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
    };
    // SAFETY: request only synchronization access, wait without blocking, and
    // close the owned handle. Access denied/unknown errors preserve the files.
    unsafe {
        let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            return GetLastError() != ERROR_INVALID_PARAMETER;
        }
        let exited = WaitForSingleObject(handle, 0) == WAIT_OBJECT_0;
        CloseHandle(handle);
        !exited
    }
}

#[cfg(not(any(unix, windows)))]
fn process_is_running(_pid: u32) -> bool {
    true
}

#[cfg(test)]
mod tests;

use crate::config;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use super::CachedPayload;

const CACHE_SCHEMA_VERSION: u32 = 1;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEnvelope<T> {
    schema_version: u32,
    namespace: String,
    key: Vec<String>,
    fetched_at_ms: u64,
    complete_through_ms: Option<u64>,
    payload: T,
}

pub(super) fn load_json<T: DeserializeOwned>(
    root: &Path,
    namespace: &str,
    key: &[String],
) -> Result<Option<CachedPayload<T>>, String> {
    let path = json_path(root, namespace, key);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(format!(
                "read {} failed: {e}",
                config::user_config_path(&path)
            ));
        }
    };
    let envelope: CacheEnvelope<T> = serde_json::from_slice(&bytes)
        .map_err(|e| format!("parse {} failed: {e}", config::user_config_path(&path)))?;
    if envelope.schema_version != CACHE_SCHEMA_VERSION
        || envelope.namespace != namespace
        || envelope.key != key
    {
        return Ok(None);
    }
    Ok(Some(CachedPayload {
        fetched_at_ms: envelope.fetched_at_ms,
        complete_through_ms: envelope.complete_through_ms,
        payload: envelope.payload,
    }))
}

pub(super) fn save_json<T: Serialize>(
    root: &Path,
    namespace: &str,
    key: &[String],
    fetched_at_ms: u64,
    complete_through_ms: Option<u64>,
    payload: &T,
) -> Result<(), String> {
    let (path, bytes) = envelope_bytes(
        root,
        namespace,
        key,
        fetched_at_ms,
        complete_through_ms,
        payload,
    )?;
    write_bytes_atomic(&path, &bytes)
}

/// Serialize a payload into its cache envelope, returning the destination path
/// and the encoded bytes. Cheap enough to run on the caller's thread; the
/// blocking write that follows is what gets handed to the cache writer.
pub(super) fn envelope_bytes<T: Serialize>(
    root: &Path,
    namespace: &str,
    key: &[String],
    fetched_at_ms: u64,
    complete_through_ms: Option<u64>,
    payload: &T,
) -> Result<(PathBuf, Vec<u8>), String> {
    let path = json_path(root, namespace, key);
    let envelope = CacheEnvelope {
        schema_version: CACHE_SCHEMA_VERSION,
        namespace: namespace.to_string(),
        key: key.to_vec(),
        fetched_at_ms,
        complete_through_ms,
        payload,
    };
    let bytes = serde_json::to_vec(&envelope)
        .map_err(|e| format!("serialize {} failed: {e}", config::user_config_path(&path)))?;
    Ok((path, bytes))
}

pub(crate) fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err(format!(
            "cache path {} has no parent",
            config::user_config_path(path)
        ));
    };
    fs::create_dir_all(parent)
        .map_err(|e| format!("create {} failed: {e}", config::user_config_path(parent)))?;

    let temp_path = temp_json_path(path);
    let write_result = (|| -> Result<(), String> {
        let mut file = open_temp_file(&temp_path)?;
        file.write_all(bytes)
            .map_err(|e| format!("write {} failed: {e}", config::user_config_path(&temp_path)))?;
        file.sync_all()
            .map_err(|e| format!("sync {} failed: {e}", config::user_config_path(&temp_path)))?;
        drop(file);
        replace_with_temp(&temp_path, path)?;
        sync_parent_dir(path);
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
}

fn open_temp_file(path: &Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    options
        .open(path)
        .map_err(|e| format!("create {} failed: {e}", config::user_config_path(path)))
}

pub(super) fn remove_json_file(path: PathBuf) -> Result<(), String> {
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!(
            "remove {} failed: {e}",
            config::user_config_path(&path)
        )),
    }
}

fn replace_with_temp(temp_path: &Path, path: &Path) -> Result<(), String> {
    match fs::rename(temp_path, path) {
        Ok(()) => Ok(()),
        Err(first_error) if cfg!(windows) && path.exists() => {
            fs::remove_file(path)
                .map_err(|e| format!("replace {} failed: {e}", config::user_config_path(path)))?;
            fs::rename(temp_path, path).map_err(|e| {
                format!(
                    "replace {} failed after removing old cache file: {e}; original error: {first_error}",
                    config::user_config_path(path)
                )
            })
        }
        Err(e) => Err(format!(
            "replace {} failed: {e}",
            config::user_config_path(path)
        )),
    }
}

fn sync_parent_dir(path: &Path) {
    if let Some(parent) = path.parent()
        && let Ok(dir) = File::open(parent)
    {
        let _ = dir.sync_all();
    }
}

pub(super) fn json_path(root: &Path, namespace: &str, key: &[String]) -> PathBuf {
    let mut path = root.join(cache_component(namespace));
    for part in key {
        path.push(cache_component(part));
    }
    path.set_extension("json");
    path
}

fn temp_json_path(path: &Path) -> PathBuf {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut file_name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| "cache.json".into());
    file_name.push(format!(".tmp.{pid}.{nanos}.{counter}"));
    path.with_file_name(file_name)
}

fn cache_component(input: &str) -> String {
    let mut component = String::new();
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_') {
            component.push(byte as char);
        } else {
            component.push('_');
            component.push_str(&format!("{byte:02x}"));
        }
    }
    if component.is_empty() {
        "_".to_string()
    } else {
        component
    }
}

#[cfg(test)]
mod tests;

use crate::agent_state::{AgentState, PersistedAgentStore};
use crate::config;

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

const MAX_SESSION_FILE_BYTES: u64 = 32 * 1024 * 1024;

// ---------------------------------------------------------------------------
// Owner-only Assistant session persistence
// ---------------------------------------------------------------------------

pub(crate) fn load_agent_state() -> AgentState {
    let Some(path) = config::assistant_sessions_path() else {
        return AgentState::default();
    };
    match load_agent_store_from_path(&path) {
        Ok(Some(store)) => AgentState::from_persisted_store(store),
        Ok(None) => AgentState::default(),
        Err(error) => AgentState {
            persistence_error: Some(error),
            ..AgentState::default()
        },
    }
}

pub(crate) async fn save_agent_store(store: PersistedAgentStore) -> Result<(), String> {
    tokio::task::spawn_blocking(move || save_agent_store_now(&store))
        .await
        .map_err(|error| format!("Assistant session save task failed: {error}"))?
}

pub(crate) fn save_agent_store_now(store: &PersistedAgentStore) -> Result<(), String> {
    let _guard = persistence_lock()
        .lock()
        .map_err(|_| "Assistant session persistence coordinator is unavailable.".to_string())?;
    let Some(path) = config::assistant_sessions_path() else {
        return Ok(());
    };
    save_agent_store_to_path(&path, store)
}

fn persistence_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn load_agent_store_from_path(path: &Path) -> Result<Option<PersistedAgentStore>, String> {
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Could not open saved Assistant sessions at {}: {error}",
                config::user_config_path(path)
            ));
        }
    };
    let metadata = file.metadata().map_err(|error| {
        format!(
            "Could not inspect saved Assistant sessions at {}: {error}",
            config::user_config_path(path)
        )
    })?;
    if metadata.len() > MAX_SESSION_FILE_BYTES {
        return Err("Saved Assistant sessions exceed the 32 MiB safety limit.".to_string());
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut bytes).map_err(|error| {
        format!(
            "Could not read saved Assistant sessions at {}: {error}",
            config::user_config_path(path)
        )
    })?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| format!("Could not parse saved Assistant sessions: {error}"))
}

fn save_agent_store_to_path(path: &Path, store: &PersistedAgentStore) -> Result<(), String> {
    let bytes = serde_json::to_vec(store)
        .map_err(|error| format!("Could not serialize Assistant sessions: {error}"))?;
    if bytes.len() as u64 > MAX_SESSION_FILE_BYTES {
        return Err("Saved Assistant sessions exceed the 32 MiB safety limit.".to_string());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "Assistant session path has no parent directory.".to_string())?;
    std::fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Could not create the Assistant session directory {}: {error}",
            config::user_config_dir()
        )
    })?;
    let temp_path = path.with_extension("json.tmp");

    let mut options = OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temp_path).map_err(|error| {
        format!(
            "Could not open {} for Assistant session persistence: {error}",
            config::user_config_path(&temp_path)
        )
    })?;
    #[cfg(target_os = "windows")]
    if let Err(error) = crate::helpers::restrict_path_to_owner(&temp_path) {
        drop(file);
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!(
            "Could not secure {}: {error}",
            config::user_config_path(&temp_path)
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|error| {
                format!(
                    "Could not secure {}: {error}",
                    config::user_config_path(&temp_path)
                )
            })?;
    }
    file.write_all(&bytes).map_err(|error| {
        format!(
            "Could not write Assistant sessions to {}: {error}",
            config::user_config_path(&temp_path)
        )
    })?;
    file.sync_all().map_err(|error| {
        format!(
            "Could not sync Assistant sessions at {}: {error}",
            config::user_config_path(&temp_path)
        )
    })?;
    drop(file);

    match std::fs::rename(&temp_path, path) {
        Ok(()) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::PermissionDenied
            ) && path.is_file() =>
        {
            std::fs::remove_file(path).map_err(|remove_error| {
                format!(
                    "Could not replace saved Assistant sessions at {}: {remove_error}",
                    config::user_config_path(path)
                )
            })?;
            std::fs::rename(&temp_path, path).map_err(|rename_error| {
                format!(
                    "Could not activate saved Assistant sessions at {}: {rename_error}",
                    config::user_config_path(path)
                )
            })
        }
        Err(error) => Err(format!(
            "Could not activate saved Assistant sessions at {}: {error}",
            config::user_config_path(path)
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_state::{PersistedAgentEntry, PersistedAgentRole, PersistedAgentSession};

    #[test]
    fn session_store_round_trips_private_chat_without_debug_output() {
        let path = test_path("round-trip");
        let store = fixture_store();

        save_agent_store_to_path(&path, &store).expect("save sessions");
        let loaded = load_agent_store_from_path(&path)
            .expect("load sessions")
            .expect("stored sessions");

        assert_eq!(loaded.active_session_id, 7);
        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(loaded.sessions[0].entries.len(), 2);
        assert_eq!(
            loaded.sessions[0].entries[0].text,
            "private portfolio question"
        );
        assert_eq!(loaded.sessions[0].context_tokens, Some(1_024));
        assert_eq!(loaded.sessions[0].context_window, Some(2_000_000));
        let _ = std::fs::remove_file(&path);
        let _ = path
            .parent()
            .and_then(|parent| std::fs::remove_dir(parent).ok());
    }

    #[cfg(unix)]
    #[test]
    fn session_store_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let path = test_path("permissions");
        save_agent_store_to_path(&path, &fixture_store()).expect("save sessions");

        let mode = std::fs::metadata(&path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        let _ = std::fs::remove_file(&path);
        let _ = path
            .parent()
            .and_then(|parent| std::fs::remove_dir(parent).ok());
    }

    fn fixture_store() -> PersistedAgentStore {
        PersistedAgentStore {
            schema_version: 1,
            active_session_id: 7,
            next_session_id: 8,
            sessions: vec![PersistedAgentSession {
                id: 7,
                title: "Portfolio review".to_string(),
                created_at_ms: 1,
                updated_at_ms: 2,
                input: String::new(),
                entries: vec![
                    PersistedAgentEntry {
                        role: PersistedAgentRole::User,
                        text: "private portfolio question".to_string(),
                    },
                    PersistedAgentEntry {
                        role: PersistedAgentRole::Assistant,
                        text: "private portfolio answer".to_string(),
                    },
                ],
                total_tokens: Some(42),
                total_cost_usd: Some(0.01),
                requested_model: Some("openrouter/auto".to_string()),
                runtime_model: Some("openrouter/auto".to_string()),
                context_tokens: Some(1_024),
                context_window: Some(2_000_000),
            }],
        }
    }

    fn test_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!(
                "kerosene-agent-sessions-{label}-{}",
                std::process::id()
            ))
            .join("assistant_sessions.json")
    }
}

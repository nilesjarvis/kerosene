use super::{KeroseneConfig, push_config_warning};

mod normalization;
mod paths;
mod persistence;
mod storage;

pub(crate) use normalization::normalize_imported_saved_layout;
pub use paths::{
    api_cache_dir, assistant_sessions_path, custom_font_path, custom_sound_path, font_storage_dir,
    journal_cache_path, sound_storage_dir,
};
pub(super) use paths::{backup_config_path, config_path, config_sidecar_prefix};
pub(crate) use paths::{in_memory_config_mode, set_in_memory_config_mode};
pub(crate) use paths::{user_config_dir, user_config_path};
pub(crate) use persistence::config_save_installed_snapshot;
#[cfg(test)]
pub(crate) use persistence::installed_config_save_error_for_test;
#[cfg(all(test, unix))]
pub(super) use persistence::write_with_restricted_permissions;
pub(super) use persistence::{load_config_from_path, save_config_to_path};

// ---------------------------------------------------------------------------
// Config Files
// ---------------------------------------------------------------------------

pub fn load_config() -> KeroseneConfig {
    let Some(path) = config_path() else {
        return KeroseneConfig::default();
    };

    let (mut config, recover_keychain_accounts) = match load_config_from_path(&path) {
        Ok(Some(config)) => (config, false),
        Ok(None) => (KeroseneConfig::default(), true),
        Err(e) => {
            push_config_warning(format!("Config load failed; defaults were used: {e}"));
            (KeroseneConfig::default(), true)
        }
    };

    normalization::normalize_loaded_config(&mut config);
    storage::load_configured_secrets(&mut config, recover_keychain_accounts);

    config
}

pub fn save_config(config: &KeroseneConfig) -> Result<(), String> {
    if paths::in_memory_config_mode() {
        return Ok(());
    }

    let Some(path) = config_path() else {
        return Err("platform config directory is unavailable".to_string());
    };
    save_config_to_path(&path, config)
}

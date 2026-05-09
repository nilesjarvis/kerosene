use super::{KeroseneConfig, push_config_warning};

mod normalization;
mod paths;
mod persistence;
mod storage;

pub use paths::journal_cache_path;
pub(super) use paths::{backup_config_path, config_path};
pub(super) use persistence::{load_config_from_path, save_config_to_path};

// ---------------------------------------------------------------------------
// Config Files
// ---------------------------------------------------------------------------

pub fn load_config() -> KeroseneConfig {
    let Some(path) = config_path() else {
        return KeroseneConfig::default();
    };

    let mut config = match load_config_from_path(&path) {
        Ok(Some(config)) => config,
        Ok(None) => KeroseneConfig::default(),
        Err(e) => {
            push_config_warning(format!("Config load failed; defaults were used: {e}"));
            KeroseneConfig::default()
        }
    };

    normalization::normalize_loaded_config(&mut config);
    storage::load_configured_secrets(&mut config);

    config
}

pub fn save_config(config: &KeroseneConfig) -> Result<(), String> {
    let Some(path) = config_path() else {
        return Err("platform config directory is unavailable".to_string());
    };
    save_config_to_path(&path, config)
}

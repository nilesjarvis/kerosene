//! Runtime-only mirror of a read-only PocketBase wallet collection.

pub(crate) mod api;

use super::AddressBookEntry;
use crate::config::RemoteWalletDatabaseConfig;
use std::collections::{HashMap, HashSet};
use std::fmt;

pub(crate) const SYNC_INTERVAL_SECS: u64 = 30;

#[derive(Default)]
pub(crate) struct RemoteWalletDatabaseState {
    pub(crate) url: String,
    pub(crate) input: String,
    pub(crate) input_error: Option<String>,
    pub(crate) entries: HashMap<String, AddressBookEntry>,
    /// Addresses introduced by this source, excluding pre-existing local wallets.
    pub(crate) added_addresses: HashSet<String>,
    pub(crate) pending_request: Option<u64>,
    pub(crate) last_synced_ms: Option<u64>,
    pub(crate) error: Option<String>,
    pub(crate) skipped: usize,
}

impl RemoteWalletDatabaseState {
    pub(crate) fn from_config(config: &RemoteWalletDatabaseConfig) -> Self {
        match api::normalize_base_url(&config.url) {
            Ok(url) => Self {
                input: url.clone(),
                url,
                ..Self::default()
            },
            Err(error) => Self {
                input: config.url.clone(),
                input_error: Some(error),
                ..Self::default()
            },
        }
    }

    pub(crate) fn status_text(&self) -> String {
        if self.url.is_empty() {
            return "Not configured".into();
        }
        if let Some(error) = &self.error {
            return if self.last_synced_ms.is_some() {
                format!("{error} · showing last sync · retrying every 30s")
            } else {
                format!("{error} · retrying every 30s")
            };
        }
        if self.pending_request.is_some() {
            return format!("Syncing · {} remote wallets", self.entries.len());
        }
        if self.last_synced_ms.is_some() {
            return format!(
                "Synced · {} remote wallets · {} unsupported addresses skipped · every 30s",
                self.entries.len(),
                self.skipped
            );
        }
        "Waiting to sync".into()
    }
}

#[derive(Clone, Default)]
pub(crate) struct RemoteWalletSnapshot {
    pub(crate) entries: HashMap<String, AddressBookEntry>,
    pub(crate) skipped: usize,
}

#[derive(Clone)]
pub(crate) struct RemoteWalletDatabaseResult(pub(crate) Result<RemoteWalletSnapshot, String>);

impl fmt::Debug for RemoteWalletDatabaseResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Ok(snapshot) => f
                .debug_struct("RemoteWalletDatabaseResult")
                .field("wallets", &snapshot.entries.len())
                .field("skipped", &snapshot.skipped)
                .finish(),
            Err(_) => f.write_str("RemoteWalletDatabaseResult(Err(<redacted>))"),
        }
    }
}

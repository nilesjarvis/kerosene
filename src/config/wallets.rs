use serde::{Deserialize, Serialize};

/// Persisted tracked-wallet entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedWalletConfig {
    /// Wallet address (0x...)
    pub address: String,
    /// Optional display label.
    #[serde(default)]
    pub label: String,
}

/// Shared wallet identity metadata used by tracker and address displays.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AddressBookEntryConfig {
    /// Wallet address (0x...).
    pub address: String,
    /// User-facing display label.
    #[serde(default)]
    pub label: String,
    /// Optional stable display color.
    #[serde(default)]
    pub color: Option<String>,
    /// Optional user tags.
    #[serde(default)]
    pub tags: Vec<String>,
}

pub const WALLET_LABELS_EXPORT_SCHEMA: &str = "kerosene.wallet_labels.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WalletLabelsExport {
    pub schema: String,
    pub exported_at_ms: u64,
    pub labels: Vec<AddressBookEntryConfig>,
}

/// Persisted wallet tracker window settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTrackerConfig {
    /// Tracked wallet addresses. Labels live in `KeroseneConfig::address_book`.
    #[serde(default)]
    pub tracked_addresses: Vec<String>,
    /// Wallet addresses hidden from the tracker while labels remain active.
    #[serde(default)]
    pub muted_addresses: Vec<String>,
    /// Tracked wallet list.
    #[serde(default)]
    pub wallets: Vec<TrackedWalletConfig>,
    /// Whether the tracker window should be reopened on startup.
    #[serde(default)]
    pub open: bool,
    /// Last window width in logical pixels.
    #[serde(default = "default_wallet_tracker_width")]
    pub width: f32,
    /// Last window height in logical pixels.
    #[serde(default = "default_wallet_tracker_height")]
    pub height: f32,
    /// Last window X position.
    #[serde(default)]
    pub x: Option<f32>,
    /// Last window Y position.
    #[serde(default)]
    pub y: Option<f32>,
}

impl Default for WalletTrackerConfig {
    fn default() -> Self {
        Self {
            tracked_addresses: Vec::new(),
            muted_addresses: Vec::new(),
            wallets: Vec::new(),
            open: false,
            width: default_wallet_tracker_width(),
            height: default_wallet_tracker_height(),
            x: None,
            y: None,
        }
    }
}

pub fn default_wallet_tracker_width() -> f32 {
    980.0
}

pub fn default_wallet_tracker_height() -> f32 {
    680.0
}

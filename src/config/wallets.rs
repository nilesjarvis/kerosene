use crate::helpers::redact_wallet_address_debug_value;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Persisted tracked-wallet entry.
#[derive(Clone, Serialize, Deserialize)]
pub struct TrackedWalletConfig {
    /// Wallet address (0x...)
    pub address: String,
    /// Optional display label.
    #[serde(default)]
    pub label: String,
}

impl fmt::Debug for TrackedWalletConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrackedWalletConfig")
            .field("address", &"<redacted>")
            .field(
                "label",
                &redact_wallet_address_debug_value(self.label.trim()),
            )
            .finish()
    }
}

/// Shared wallet identity metadata used by tracker and address displays.
#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
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

impl fmt::Debug for AddressBookEntryConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AddressBookEntryConfig")
            .field("address", &"<redacted>")
            .field(
                "label",
                &redact_wallet_address_debug_value(self.label.trim()),
            )
            .field("color", &self.color)
            .field("tags", &RedactedWalletTags(self.tags.len()))
            .finish()
    }
}

pub const WALLET_LABELS_EXPORT_SCHEMA: &str = "kerosene.wallet_labels.v1";

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct WalletLabelsExport {
    pub schema: String,
    pub exported_at_ms: u64,
    pub labels: Vec<AddressBookEntryConfig>,
}

impl fmt::Debug for WalletLabelsExport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletLabelsExport")
            .field("schema", &self.schema)
            .field("exported_at_ms", &self.exported_at_ms)
            .field("labels", &self.labels)
            .finish()
    }
}

/// Persisted wallet tracker window settings.
#[derive(Clone, Serialize, Deserialize)]
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

impl fmt::Debug for WalletTrackerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletTrackerConfig")
            .field(
                "tracked_addresses",
                &RedactedWalletAddressList(self.tracked_addresses.len()),
            )
            .field(
                "muted_addresses",
                &RedactedWalletAddressList(self.muted_addresses.len()),
            )
            .field("wallets", &self.wallets)
            .field("open", &self.open)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("x", &self.x)
            .field("y", &self.y)
            .finish()
    }
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

struct RedactedWalletAddressList(usize);

impl fmt::Debug for RedactedWalletAddressList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} redacted>", self.0)
    }
}

struct RedactedWalletTags(usize);

impl fmt::Debug for RedactedWalletTags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} redacted>", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AddressBookEntryConfig, TrackedWalletConfig, WALLET_LABELS_EXPORT_SCHEMA,
        WalletLabelsExport, WalletTrackerConfig,
    };

    const ADDRESS_A: &str = "0x1111111111111111111111111111111111111111";
    const ADDRESS_B: &str = "0x2222222222222222222222222222222222222222";
    const ADDRESS_C: &str = "0x3333333333333333333333333333333333333333";

    #[test]
    fn wallet_config_debug_redacts_addresses() {
        let cfg = WalletTrackerConfig {
            tracked_addresses: vec![ADDRESS_A.to_string(), ADDRESS_B.to_string()],
            muted_addresses: vec![ADDRESS_C.to_string()],
            wallets: vec![TrackedWalletConfig {
                address: ADDRESS_A.to_string(),
                label: "Whale".to_string(),
            }],
            open: true,
            width: 900.0,
            height: 700.0,
            x: Some(12.0),
            y: Some(34.0),
        };

        let rendered = format!("{cfg:?}");

        for address in [ADDRESS_A, ADDRESS_B, ADDRESS_C] {
            assert!(!rendered.contains(address));
        }
        assert!(rendered.contains("tracked_addresses: <2 redacted>"));
        assert!(rendered.contains("muted_addresses: <1 redacted>"));
        assert!(rendered.contains("Whale"));
        assert!(rendered.contains("open: true"));
    }

    #[test]
    fn wallet_labels_export_debug_redacts_entry_addresses() {
        let export = WalletLabelsExport {
            schema: WALLET_LABELS_EXPORT_SCHEMA.to_string(),
            exported_at_ms: 42,
            labels: vec![AddressBookEntryConfig {
                address: ADDRESS_A.to_string(),
                label: ADDRESS_B.to_string(),
                color: Some("#ff00ff".to_string()),
                tags: vec!["desk".to_string(), ADDRESS_C.to_string()],
            }],
        };

        let rendered = format!("{export:?}");

        assert!(!rendered.contains(ADDRESS_A));
        assert!(!rendered.contains(ADDRESS_B));
        assert!(!rendered.contains(ADDRESS_C));
        assert!(!rendered.contains("desk"));
        assert!(rendered.contains("<redacted>"));
        assert!(rendered.contains("tags: <2 redacted>"));
        assert!(rendered.contains("#ff00ff"));
    }
}

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
                &(!self.label.trim().is_empty()).then_some("<redacted>"),
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
                &(!self.label.trim().is_empty()).then_some("<redacted>"),
            )
            .field("color", &self.color.as_ref().map(|_| "<redacted>"))
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
            .field(
                "schema_supported",
                &(self.schema == WALLET_LABELS_EXPORT_SCHEMA),
            )
            .field("exported_at_ms", &format_args!("<redacted>"))
            .field("labels_len", &self.labels.len())
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

/// Persisted member of a tradable wallet cluster.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct WalletClusterMemberConfig {
    /// Stable account profile secret id. This references an existing saved
    /// account profile; it never contains the agent key itself.
    ///
    /// Defaulted so a single malformed/legacy member entry deserializes to an
    /// empty id (then dropped by `WalletCluster::from_config`) instead of
    /// failing the whole `KeroseneConfig` parse and resetting every setting.
    #[serde(default)]
    pub profile_secret_id: String,
    /// Relative allocation weight for aggregate orders.
    #[serde(default = "default_wallet_cluster_member_weight")]
    pub weight: f64,
}

impl fmt::Debug for WalletClusterMemberConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletClusterMemberConfig")
            .field("profile_secret_id", &"<redacted>")
            .field("weight", &self.weight)
            .finish()
    }
}

/// Persisted tradable wallet cluster.
#[derive(Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct WalletClusterConfig {
    /// Stable id for the cluster.
    ///
    /// Defaulted so a single malformed/legacy cluster entry deserializes to an
    /// empty id (then dropped by `WalletClusterState::from_config`) instead of
    /// failing the whole `KeroseneConfig` parse and resetting every setting.
    #[serde(default)]
    pub id: String,
    /// User-facing name.
    #[serde(default)]
    pub name: String,
    /// Account profile references.
    #[serde(default)]
    pub members: Vec<WalletClusterMemberConfig>,
}

impl fmt::Debug for WalletClusterConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletClusterConfig")
            .field("id", &"<redacted>")
            .field("name", &redact_wallet_address_debug_value(self.name.trim()))
            .field("members", &RedactedWalletClusterMembers(self.members.len()))
            .finish()
    }
}

/// Persisted wallet cluster window and cluster definitions.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct WalletClustersConfig {
    #[serde(default)]
    pub clusters: Vec<WalletClusterConfig>,
    #[serde(default)]
    pub selected_cluster_id: Option<String>,
    #[serde(default)]
    pub open: bool,
    #[serde(default = "default_wallet_clusters_width")]
    pub width: f32,
    #[serde(default = "default_wallet_clusters_height")]
    pub height: f32,
    #[serde(default)]
    pub x: Option<f32>,
    #[serde(default)]
    pub y: Option<f32>,
}

impl Default for WalletClustersConfig {
    fn default() -> Self {
        Self {
            clusters: Vec::new(),
            selected_cluster_id: None,
            open: false,
            width: default_wallet_clusters_width(),
            height: default_wallet_clusters_height(),
            x: None,
            y: None,
        }
    }
}

impl fmt::Debug for WalletClustersConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletClustersConfig")
            .field(
                "clusters",
                &RedactedWalletClusterMembers(self.clusters.len()),
            )
            .field(
                "selected_cluster_id",
                &self.selected_cluster_id.as_ref().map(|_| "<redacted>"),
            )
            .field("open", &self.open)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("x", &self.x)
            .field("y", &self.y)
            .finish()
    }
}

pub fn default_wallet_cluster_member_weight() -> f64 {
    1.0
}

pub fn default_wallet_clusters_width() -> f32 {
    1180.0
}

pub fn default_wallet_clusters_height() -> f32 {
    760.0
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

struct RedactedWalletClusterMembers(usize);

impl fmt::Debug for RedactedWalletClusterMembers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} redacted>", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AddressBookEntryConfig, TrackedWalletConfig, WALLET_LABELS_EXPORT_SCHEMA,
        WalletClustersConfig, WalletLabelsExport, WalletTrackerConfig,
    };

    const ADDRESS_A: &str = "0x1111111111111111111111111111111111111111";
    const ADDRESS_B: &str = "0x2222222222222222222222222222222222222222";
    const ADDRESS_C: &str = "0x3333333333333333333333333333333333333333";

    #[test]
    fn wallet_config_debug_redacts_addresses_and_labels_without_changing_wire_data() {
        const LABEL: &str = "private-tracked-wallet-label-sentinel";
        let cfg = WalletTrackerConfig {
            tracked_addresses: vec![ADDRESS_A.to_string(), ADDRESS_B.to_string()],
            muted_addresses: vec![ADDRESS_C.to_string()],
            wallets: vec![TrackedWalletConfig {
                address: ADDRESS_A.to_string(),
                label: LABEL.to_string(),
            }],
            open: true,
            width: 900.0,
            height: 700.0,
            x: Some(12.0),
            y: Some(34.0),
        };
        let wire_before = serde_json::to_value(&cfg).expect("serialize wallet tracker config");

        let rendered = format!("{cfg:?}");

        for address in [ADDRESS_A, ADDRESS_B, ADDRESS_C] {
            assert!(!rendered.contains(address));
        }
        assert!(rendered.contains("tracked_addresses: <2 redacted>"));
        assert!(rendered.contains("muted_addresses: <1 redacted>"));
        assert!(rendered.contains("label: Some(\"<redacted>\")"));
        assert!(!rendered.contains(LABEL));
        assert!(rendered.contains("open: true"));
        assert_eq!(
            serde_json::to_value(&cfg).expect("serialize wallet tracker config after formatting"),
            wire_before
        );
    }

    #[test]
    fn address_book_config_debug_is_structural_and_preserves_wire_data() {
        const LABEL: &str = "private-address-book-label-sentinel";
        const COLOR: &str = "#a1b2c3";
        const TAG: &str = "private-address-book-tag-sentinel";
        let entry = AddressBookEntryConfig {
            address: ADDRESS_A.to_string(),
            label: LABEL.to_string(),
            color: Some(COLOR.to_string()),
            tags: vec![TAG.to_string(), ADDRESS_B.to_string()],
        };
        let wire_before = serde_json::to_value(&entry).expect("serialize address-book entry");

        let rendered = format!("{entry:?}");

        assert!(rendered.contains("address: \"<redacted>\""), "{rendered}");
        assert!(
            rendered.contains("label: Some(\"<redacted>\")"),
            "{rendered}"
        );
        assert!(
            rendered.contains("color: Some(\"<redacted>\")"),
            "{rendered}"
        );
        assert!(rendered.contains("tags: <2 redacted>"), "{rendered}");
        for sensitive in [ADDRESS_A, ADDRESS_B, LABEL, COLOR, TAG] {
            assert!(
                !rendered.contains(sensitive),
                "{sensitive} leaked in {rendered}"
            );
        }
        assert_eq!(
            serde_json::to_value(&entry).expect("serialize address-book entry after formatting"),
            wire_before
        );
    }

    #[test]
    fn malformed_cluster_entries_deserialize_without_failing_whole_config() {
        // A cluster missing `id` and a member missing `profile_secret_id` must
        // not abort deserialization (which load_config turns into a full reset
        // to defaults). They should default to empty strings and survive parse.
        let json = r#"{
            "clusters": [
                { "name": "No Id", "members": [ { "weight": 2.0 } ] },
                { "id": "good", "name": "Good", "members": [] }
            ],
            "open": true
        }"#;

        let config: WalletClustersConfig =
            serde_json::from_str(json).expect("malformed cluster entries must still parse");

        assert_eq!(config.clusters.len(), 2);
        assert_eq!(config.clusters[0].id, "");
        assert_eq!(config.clusters[0].members[0].profile_secret_id, "");
        assert_eq!(config.clusters[0].members[0].weight, 2.0);
        assert_eq!(config.clusters[1].id, "good");
        assert!(config.open);
    }

    #[test]
    fn wallet_labels_export_debug_is_structural_and_preserves_wire_payload() {
        const EXPORTED_AT_MS: u64 = 9_123_456_789;
        const LABEL: &str = "private-wallet-label-sentinel";
        const COLOR: &str = "#a1b2c3";
        const TAG: &str = "private-wallet-tag-sentinel";
        let export = WalletLabelsExport {
            schema: WALLET_LABELS_EXPORT_SCHEMA.to_string(),
            exported_at_ms: EXPORTED_AT_MS,
            labels: vec![AddressBookEntryConfig {
                address: ADDRESS_A.to_string(),
                label: LABEL.to_string(),
                color: Some(COLOR.to_string()),
                tags: vec![TAG.to_string(), ADDRESS_C.to_string()],
            }],
        };
        let wire_before = serde_json::to_value(&export).expect("serialize wallet labels");

        let rendered = format!("{export:?}");

        assert!(rendered.contains("schema_supported: true"), "{rendered}");
        assert!(rendered.contains("labels_len: 1"), "{rendered}");
        assert!(
            rendered.contains("exported_at_ms: <redacted>"),
            "{rendered}"
        );
        assert!(!rendered.contains(ADDRESS_A));
        assert!(!rendered.contains(ADDRESS_C));
        assert!(!rendered.contains(LABEL));
        assert!(!rendered.contains(COLOR));
        assert!(!rendered.contains(TAG));
        assert!(!rendered.contains(&EXPORTED_AT_MS.to_string()));
        assert!(!rendered.contains(WALLET_LABELS_EXPORT_SCHEMA));
        assert_eq!(
            serde_json::to_value(&export).expect("serialize wallet labels after formatting"),
            wire_before
        );
    }
}

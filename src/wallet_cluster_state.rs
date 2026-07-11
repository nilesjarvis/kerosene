use crate::account::WalletDetailsData;
use crate::config::{
    WalletClusterConfig, WalletClusterMemberConfig, WalletClustersConfig,
    default_wallet_cluster_member_weight, default_wallet_clusters_height,
    default_wallet_clusters_width,
};
use crate::helpers::parse_finite_number;
use crate::read_data_provider::ReadDataRequestContext;
use crate::signing::OrderKind;

use iced::window;
use std::collections::{HashMap, VecDeque};
use std::fmt;

pub(crate) const MAX_WALLET_CLUSTER_MEMBERS: usize = 10;
pub(crate) const MAX_WALLET_CLUSTER_EXECUTIONS: usize = 20;

// ---------------------------------------------------------------------------
// Runtime Cluster Models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WalletClusterCloseSide {
    Long,
    Short,
}

impl WalletClusterCloseSide {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Long => "Long",
            Self::Short => "Short",
        }
    }

    pub(crate) fn is_buy_to_close(self) -> bool {
        matches!(self, Self::Short)
    }
}

#[derive(Clone)]
pub(crate) struct WalletClusterMember {
    pub(crate) profile_secret_id: String,
    pub(crate) weight: f64,
    pub(crate) weight_input: String,
}

impl fmt::Debug for WalletClusterMember {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletClusterMember")
            .field("profile_secret_id", &"<redacted>")
            .field("weight", &"<redacted>")
            .field("weight_input", &"<redacted>")
            .finish()
    }
}

impl WalletClusterMember {
    fn from_config(config: &WalletClusterMemberConfig) -> Self {
        let weight = normalize_member_weight(config.weight);
        Self {
            profile_secret_id: config.profile_secret_id.clone(),
            weight,
            weight_input: format_weight_input(weight),
        }
    }

    fn to_config(&self) -> WalletClusterMemberConfig {
        WalletClusterMemberConfig {
            profile_secret_id: self.profile_secret_id.clone(),
            weight: self.weight,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct WalletCluster {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) members: Vec<WalletClusterMember>,
}

impl fmt::Debug for WalletCluster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletCluster")
            .field("id", &"<redacted>")
            .field("name", &"<redacted>")
            .field(
                "members",
                &format_args!("<{} redacted>", self.members.len()),
            )
            .finish()
    }
}

impl WalletCluster {
    fn from_config(config: &WalletClusterConfig) -> Self {
        let mut members = Vec::new();
        for member in &config.members {
            if member.profile_secret_id.trim().is_empty()
                || members.iter().any(|existing: &WalletClusterMember| {
                    existing.profile_secret_id == member.profile_secret_id
                })
            {
                continue;
            }
            if members.len() >= MAX_WALLET_CLUSTER_MEMBERS {
                break;
            }
            members.push(WalletClusterMember::from_config(member));
        }

        Self {
            id: config.id.clone(),
            name: config.name.clone(),
            members,
        }
    }

    fn to_config(&self) -> WalletClusterConfig {
        WalletClusterConfig {
            id: self.id.clone(),
            name: self.name.clone(),
            members: self
                .members
                .iter()
                .map(WalletClusterMember::to_config)
                .collect(),
        }
    }

    pub(crate) fn display_name(&self) -> String {
        let name = self.name.trim();
        if name.is_empty() {
            "Untitled cluster".to_string()
        } else {
            name.to_string()
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct WalletClusterMemberData {
    pub(crate) address: String,
    pub(crate) data: Option<WalletDetailsData>,
    pub(crate) loading: bool,
    pub(crate) loading_context: Option<ReadDataRequestContext>,
    pub(crate) error: Option<String>,
    /// Timestamp of the last refresh that delivered fresh *positions*
    /// (full REST snapshot or an `AllDexPositions` ws frame). Non-position
    /// frames (open orders, spot balances) must NOT bump this, so the
    /// close-action freshness gate is never fooled into sizing a reduce-only
    /// close from stale position data.
    pub(crate) positions_refreshed_ms: Option<u64>,
    pub(crate) stale: bool,
}

impl fmt::Debug for WalletClusterMemberData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletClusterMemberData")
            .field("address", &"<redacted>")
            .field("data", &self.data.as_ref().map(|_| "<redacted>"))
            .field("loading", &self.loading)
            .field("loading_context", &self.loading_context)
            .field("error", &self.error.as_ref().map(|_| "<redacted>"))
            .field("positions_refreshed_ms", &self.positions_refreshed_ms)
            .field("stale", &self.stale)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WalletClusterExecutionKind {
    Order,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WalletClusterLegStatus {
    Pending,
    Confirmed,
    Failed,
    Uncertain,
    Checking,
}

impl WalletClusterLegStatus {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Confirmed => "Confirmed",
            Self::Failed => "Failed",
            Self::Uncertain => "Uncertain",
            Self::Checking => "Checking",
        }
    }

    pub(crate) fn is_terminal(self) -> bool {
        matches!(self, Self::Confirmed | Self::Failed | Self::Uncertain)
    }
}

#[derive(Clone)]
pub(crate) struct WalletClusterExecutionLeg {
    pub(crate) profile_secret_id: String,
    pub(crate) address: String,
    pub(crate) label: String,
    pub(crate) symbol: String,
    pub(crate) is_buy: bool,
    pub(crate) size: String,
    pub(crate) price: String,
    pub(crate) cloid: String,
    pub(crate) status: WalletClusterLegStatus,
    pub(crate) message: String,
}

impl fmt::Debug for WalletClusterExecutionLeg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletClusterExecutionLeg")
            .field("profile_secret_id", &"<redacted>")
            .field("address", &"<redacted>")
            .field("label", &"<redacted>")
            .field("symbol", &"<redacted>")
            .field("is_buy", &self.is_buy)
            .field("size", &"<redacted>")
            .field("price", &"<redacted>")
            .field("cloid", &"<redacted>")
            .field("status", &self.status)
            .field("message", &"<redacted>")
            .finish()
    }
}

#[derive(Clone)]
pub(crate) struct WalletClusterExecution {
    pub(crate) id: u64,
    pub(crate) cluster_name: String,
    pub(crate) kind: WalletClusterExecutionKind,
    pub(crate) symbol: String,
    pub(crate) order_kind: OrderKind,
    pub(crate) created_at_ms: u64,
    pub(crate) legs: Vec<WalletClusterExecutionLeg>,
}

impl fmt::Debug for WalletClusterExecution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletClusterExecution")
            .field("id", &self.id)
            .field("cluster_name", &"<redacted>")
            .field("kind", &self.kind)
            .field("symbol", &"<redacted>")
            .field("order_kind", &self.order_kind)
            .field("created_at_ms", &self.created_at_ms)
            .field("legs", &format_args!("len={}", self.legs.len()))
            .finish()
    }
}

impl WalletClusterExecution {
    pub(crate) fn completed_count(&self) -> usize {
        self.legs
            .iter()
            .filter(|leg| leg.status.is_terminal())
            .count()
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.completed_count() >= self.legs.len()
    }

    pub(crate) fn problem_count(&self) -> usize {
        self.legs
            .iter()
            .filter(|leg| {
                matches!(
                    leg.status,
                    WalletClusterLegStatus::Failed | WalletClusterLegStatus::Uncertain
                )
            })
            .count()
    }
}

pub(crate) struct WalletClusterPositionMember {
    pub(crate) profile_secret_id: String,
    pub(crate) address: String,
    pub(crate) label: String,
    pub(crate) dex: String,
    pub(crate) size: f64,
    pub(crate) entry_price: Option<f64>,
    pub(crate) value: Option<f64>,
    pub(crate) unrealized_pnl: Option<f64>,
}

pub(crate) struct WalletClusterPositionSummary {
    pub(crate) symbol: String,
    pub(crate) net_size: f64,
    pub(crate) long_size: f64,
    pub(crate) short_size: f64,
    pub(crate) value: Option<f64>,
    pub(crate) unrealized_pnl: Option<f64>,
    pub(crate) members: Vec<WalletClusterPositionMember>,
}

impl WalletClusterPositionSummary {
    pub(crate) fn has_long(&self) -> bool {
        self.long_size > 1e-12
    }

    pub(crate) fn has_short(&self) -> bool {
        self.short_size > 1e-12
    }
}

pub(crate) struct WalletClusterState {
    pub(crate) window_id: Option<window::Id>,
    pub(crate) open: bool,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) x: Option<f32>,
    pub(crate) y: Option<f32>,
    pub(crate) clusters: Vec<WalletCluster>,
    pub(crate) selected_cluster_id: Option<String>,
    pub(crate) new_cluster_name_input: String,
    pub(crate) order_price: String,
    pub(crate) order_quantity: String,
    pub(crate) order_quantity_is_usd: bool,
    pub(crate) order_kind: OrderKind,
    pub(crate) reduce_only: bool,
    pub(crate) status: Option<(String, bool)>,
    pub(crate) member_data: HashMap<String, WalletClusterMemberData>,
    pub(crate) executions: VecDeque<WalletClusterExecution>,
    pub(crate) next_execution_id: u64,
}

impl fmt::Debug for WalletClusterState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletClusterState")
            .field("window_id", &self.window_id)
            .field("open", &self.open)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("x", &self.x)
            .field("y", &self.y)
            .field("clusters", &format_args!("len={}", self.clusters.len()))
            .field(
                "selected_cluster_id",
                &self.selected_cluster_id.as_ref().map(|_| "<redacted>"),
            )
            .field("new_cluster_name_input", &"<redacted>")
            .field("order_price", &"<redacted>")
            .field("order_quantity", &"<redacted>")
            .field("order_quantity_is_usd", &self.order_quantity_is_usd)
            .field("order_kind", &self.order_kind)
            .field("reduce_only", &self.reduce_only)
            .field("status", &self.status.as_ref().map(|_| "<redacted>"))
            .field(
                "member_data",
                &format_args!("len={}", self.member_data.len()),
            )
            .field("executions", &format_args!("len={}", self.executions.len()))
            .field("next_execution_id", &self.next_execution_id)
            .finish()
    }
}

impl WalletClusterState {
    pub(crate) fn from_config(config: &WalletClustersConfig) -> Self {
        let clusters: Vec<_> = config
            .clusters
            .iter()
            .filter(|cluster| !cluster.id.trim().is_empty())
            .map(WalletCluster::from_config)
            .collect();
        let selected_cluster_id = config
            .selected_cluster_id
            .as_ref()
            .filter(|id| clusters.iter().any(|cluster| &cluster.id == *id))
            .cloned()
            .or_else(|| clusters.first().map(|cluster| cluster.id.clone()));

        Self {
            window_id: None,
            open: config.open,
            width: config.width,
            height: config.height,
            x: config.x,
            y: config.y,
            clusters,
            selected_cluster_id,
            new_cluster_name_input: String::new(),
            order_price: String::new(),
            order_quantity: String::new(),
            order_quantity_is_usd: true,
            order_kind: OrderKind::Market,
            reduce_only: false,
            status: None,
            member_data: HashMap::new(),
            executions: VecDeque::new(),
            next_execution_id: 1,
        }
    }

    pub(crate) fn to_config(&self) -> WalletClustersConfig {
        WalletClustersConfig {
            clusters: self.clusters.iter().map(WalletCluster::to_config).collect(),
            selected_cluster_id: self.selected_cluster_id.clone(),
            open: self.open,
            width: self.width,
            height: self.height,
            x: self.x,
            y: self.y,
        }
    }

    pub(crate) fn selected_cluster(&self) -> Option<&WalletCluster> {
        let selected = self.selected_cluster_id.as_deref()?;
        self.clusters.iter().find(|cluster| cluster.id == selected)
    }

    pub(crate) fn selected_cluster_mut(&mut self) -> Option<&mut WalletCluster> {
        let selected = self.selected_cluster_id.as_deref()?;
        self.clusters
            .iter_mut()
            .find(|cluster| cluster.id == selected)
    }

    pub(crate) fn has_pending_execution(&self) -> bool {
        self.executions
            .iter()
            .any(|execution| !execution.is_complete())
    }

    pub(crate) fn push_execution(&mut self, execution: WalletClusterExecution) {
        self.executions.push_front(execution);
        while self.executions.len() > MAX_WALLET_CLUSTER_EXECUTIONS {
            // Evict the oldest *completed* execution. Never drop one still in
            // flight: has_pending_execution() and late leg results depend on
            // it, and dropping it would unblock account changes mid-execution.
            let Some(remove_index) = self
                .executions
                .iter()
                .rposition(|execution| execution.is_complete())
            else {
                break;
            };
            self.executions.remove(remove_index);
        }
    }
}

pub(crate) fn normalize_member_weight(weight: f64) -> f64 {
    // Preserve a deliberately-disabled member (weight 0); only coerce
    // non-finite or negative weights to the default. Coercing 0 -> default
    // would silently re-enable, at full weight, a wallet the user excluded —
    // placing a real order on it after the next restart.
    if weight.is_finite() && weight >= 0.0 {
        weight
    } else {
        default_wallet_cluster_member_weight()
    }
}

pub(crate) fn parse_member_weight(input: &str) -> Option<f64> {
    parse_finite_number(input).filter(|weight| weight.is_finite() && *weight >= 0.0)
}

pub(crate) fn format_weight_input(weight: f64) -> String {
    let mut value = format!("{weight:.4}");
    while value.contains('.') && value.ends_with('0') {
        value.pop();
    }
    if value.ends_with('.') {
        value.pop();
    }
    value
}

pub(crate) fn wallet_cluster_window_settings(
    state: &WalletClusterState,
    custom_chrome: bool,
) -> window::Settings {
    window::Settings {
        size: iced::Size::new(
            state.width.max(default_wallet_clusters_width() * 0.6),
            state.height.max(default_wallet_clusters_height() * 0.6),
        ),
        position: state
            .x
            .zip(state.y)
            .map(|(x, y)| window::Position::Specific(iced::Point::new(x, y)))
            .unwrap_or_else(|| window::Position::Centered),
        ..crate::window_chrome::settings(custom_chrome)
    }
}

pub(crate) fn cluster_button_label(order_kind: OrderKind) -> &'static str {
    match order_kind {
        OrderKind::Market => "Market",
        OrderKind::Limit => "Limit",
        OrderKind::LimitIoc => "IOC",
        OrderKind::Chase => "Chase",
        OrderKind::Twap => "TWAP",
    }
}

pub(crate) fn cluster_order_kind_options() -> [OrderKind; 3] {
    [OrderKind::Market, OrderKind::Limit, OrderKind::LimitIoc]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_member_weights_normalize_to_default() {
        for weight in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
            assert_eq!(normalize_member_weight(weight), 1.0);
        }
        assert_eq!(normalize_member_weight(2.5), 2.5);
        // A deliberately disabled member (weight 0) must survive normalization;
        // coercing it to the default would re-enable an excluded wallet.
        assert_eq!(normalize_member_weight(0.0), 0.0);
    }

    #[test]
    fn disabled_member_weight_survives_config_round_trip() {
        let cfg = WalletClustersConfig {
            clusters: vec![WalletClusterConfig {
                id: "cluster".to_string(),
                name: "Main".to_string(),
                members: vec![WalletClusterMemberConfig {
                    profile_secret_id: "disabled".to_string(),
                    weight: 0.0,
                }],
            }],
            selected_cluster_id: Some("cluster".to_string()),
            ..WalletClustersConfig::default()
        };

        let state = WalletClusterState::from_config(&cfg);
        assert_eq!(state.clusters[0].members[0].weight, 0.0);
        // And it persists back as 0, not the default 1.0.
        assert_eq!(state.to_config().clusters[0].members[0].weight, 0.0);
    }

    #[test]
    fn push_execution_evicts_oldest_complete_but_keeps_pending() {
        let mut state = WalletClusterState::from_config(&WalletClustersConfig::default());

        let make = |id: u64, terminal: bool| WalletClusterExecution {
            id,
            cluster_name: "c".to_string(),
            kind: WalletClusterExecutionKind::Order,
            symbol: "BTC".to_string(),
            order_kind: OrderKind::Market,
            created_at_ms: id,
            legs: vec![WalletClusterExecutionLeg {
                profile_secret_id: "p".to_string(),
                address: "0x".to_string(),
                label: "l".to_string(),
                symbol: "BTC".to_string(),
                is_buy: true,
                size: "1".to_string(),
                price: "1".to_string(),
                cloid: format!("0x{id}"),
                status: if terminal {
                    WalletClusterLegStatus::Confirmed
                } else {
                    WalletClusterLegStatus::Pending
                },
                message: String::new(),
            }],
        };

        // Oldest execution is still pending; fill the history past the cap with
        // completed ones. The pending execution must never be evicted.
        state.push_execution(make(0, false));
        for id in 1..=(MAX_WALLET_CLUSTER_EXECUTIONS as u64 + 5) {
            state.push_execution(make(id, true));
        }

        assert!(state.executions.len() <= MAX_WALLET_CLUSTER_EXECUTIONS + 1);
        assert!(
            state.executions.iter().any(|execution| execution.id == 0),
            "pending execution was evicted"
        );
        assert!(state.has_pending_execution());
    }

    #[test]
    fn member_weight_input_accepts_zero_for_disabled_members() {
        assert_eq!(parse_member_weight("0"), Some(0.0));
        assert_eq!(parse_member_weight("2.25"), Some(2.25));
        assert_eq!(parse_member_weight("-1"), None);
        assert_eq!(parse_member_weight("nan"), None);
    }

    #[test]
    fn cluster_runtime_debug_redacts_identity_and_weight_without_changing_values() {
        const CLUSTER_ID: &str = "private-runtime-cluster-id-sentinel";
        const CLUSTER_NAME: &str = "private-runtime-cluster-name-sentinel";
        const PROFILE_ID: &str = "private-runtime-profile-id-sentinel";
        const WEIGHT_BITS: u64 = 0x4009_21fb_5444_2d18;
        let state = WalletClusterState::from_config(&WalletClustersConfig {
            clusters: vec![WalletClusterConfig {
                id: CLUSTER_ID.to_string(),
                name: CLUSTER_NAME.to_string(),
                members: vec![WalletClusterMemberConfig {
                    profile_secret_id: PROFILE_ID.to_string(),
                    weight: f64::from_bits(WEIGHT_BITS),
                }],
            }],
            selected_cluster_id: Some(CLUSTER_ID.to_string()),
            ..WalletClustersConfig::default()
        });
        let cluster = &state.clusters[0];

        let cluster_debug = format!("{cluster:?}");
        let member_debug = format!("{:?}", cluster.members[0]);

        for sensitive in [CLUSTER_ID, CLUSTER_NAME, PROFILE_ID] {
            assert!(
                !cluster_debug.contains(sensitive),
                "{sensitive} leaked in {cluster_debug}"
            );
            assert!(
                !member_debug.contains(sensitive),
                "{sensitive} leaked in {member_debug}"
            );
        }
        assert!(
            member_debug.contains("weight: \"<redacted>\""),
            "{member_debug}"
        );
        assert_eq!(cluster.id, CLUSTER_ID);
        assert_eq!(cluster.name, CLUSTER_NAME);
        assert_eq!(cluster.members[0].profile_secret_id, PROFILE_ID);
        assert_eq!(cluster.members[0].weight.to_bits(), WEIGHT_BITS);
    }

    #[test]
    fn execution_leg_debug_redacts_lifecycle_message() {
        const RAW_CLOID: &str = "0x0000000000000000000000000000000f";
        const LABEL: &str = "private-execution-member-label-sentinel";
        const CLUSTER_NAME: &str = "private-execution-cluster-name-sentinel";
        let message = format!("orderStatus says cancel {RAW_CLOID} if needed");
        let leg = WalletClusterExecutionLeg {
            profile_secret_id: "profile-secret".to_string(),
            address: "0x1111111111111111111111111111111111111111".to_string(),
            label: LABEL.to_string(),
            symbol: "BTC".to_string(),
            is_buy: true,
            size: "1".to_string(),
            price: "100".to_string(),
            cloid: RAW_CLOID.to_string(),
            status: WalletClusterLegStatus::Uncertain,
            message: message.clone(),
        };

        let rendered = format!("{leg:?}");

        assert!(rendered.contains("message: \"<redacted>\""));
        assert!(!rendered.contains(RAW_CLOID));
        assert!(!rendered.contains(LABEL));
        assert_eq!(leg.label, LABEL);
        assert_eq!(leg.message, message, "stored UI message must remain intact");

        let execution = WalletClusterExecution {
            id: 42,
            cluster_name: CLUSTER_NAME.to_string(),
            kind: WalletClusterExecutionKind::Order,
            symbol: "BTC".to_string(),
            order_kind: OrderKind::Limit,
            created_at_ms: 123,
            legs: vec![leg],
        };
        let execution_debug = format!("{execution:?}");

        assert!(
            execution_debug.contains("cluster_name: \"<redacted>\""),
            "{execution_debug}"
        );
        assert!(!execution_debug.contains(CLUSTER_NAME), "{execution_debug}");
        assert_eq!(execution.cluster_name, CLUSTER_NAME);
        assert_eq!(execution.legs[0].label, LABEL);
    }

    #[test]
    fn cluster_state_config_round_trip_redacts_debug() {
        let cfg = WalletClustersConfig {
            clusters: vec![WalletClusterConfig {
                id: "cluster-secret".to_string(),
                name: "Main".to_string(),
                members: vec![WalletClusterMemberConfig {
                    profile_secret_id: "profile-secret".to_string(),
                    weight: 2.0,
                }],
            }],
            selected_cluster_id: Some("cluster-secret".to_string()),
            open: true,
            width: 900.0,
            height: 700.0,
            x: Some(1.0),
            y: Some(2.0),
        };

        let state = WalletClusterState::from_config(&cfg);
        let rendered = format!("{state:?}");

        assert!(!rendered.contains("cluster-secret"));
        assert!(!rendered.contains("profile-secret"));
        assert_eq!(state.to_config().clusters[0].members[0].weight, 2.0);
    }

    #[test]
    fn close_side_maps_to_exchange_side() {
        assert!(!WalletClusterCloseSide::Long.is_buy_to_close());
        assert!(WalletClusterCloseSide::Short.is_buy_to_close());
    }
}

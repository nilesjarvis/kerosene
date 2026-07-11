mod pane_kind_wire;
mod widget_padding_wire;

use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::BTreeMap, fmt};

use super::{
    ChartConfig, CustomThemeConfig, LiveWatchlistConfig, OrderBookConfig, OrderPresetsConfig,
    PositioningInfoConfig, SpaghettiChartConfig, default_custom_themes, default_order_kind,
    default_symbol, default_timeframe, default_true, default_widget_padding,
    normalize_pane_split_ratio, normalize_widget_padding,
};

/// Persisted axis for a pane split.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Default)]
pub enum AxisConfig {
    #[default]
    Horizontal,
    Vertical,
}

impl AxisConfig {
    fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "Horizontal" => Some(Self::Horizontal),
            "Vertical" => Some(Self::Vertical),
            _ => None,
        }
    }

    fn config_value(self) -> &'static str {
        match self {
            Self::Horizontal => "Horizontal",
            Self::Vertical => "Vertical",
        }
    }
}

impl<'de> Deserialize<'de> for AxisConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_config_value(&value).unwrap_or_else(|| {
            let default = Self::default();
            crate::config::push_config_warning(format!(
                "Unknown pane split axis {value:?} in config; using {}",
                default.config_value()
            ));
            default
        }))
    }
}

/// Persisted bottom tab selection.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Default)]
pub enum BottomTabConfig {
    #[default]
    Positions,
    OpenOrders,
    Balances,
    TradeHistory,
    FundingHistory,
}

impl BottomTabConfig {
    fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "Positions" => Some(Self::Positions),
            "OpenOrders" => Some(Self::OpenOrders),
            "Balances" => Some(Self::Balances),
            "TradeHistory" => Some(Self::TradeHistory),
            "FundingHistory" => Some(Self::FundingHistory),
            _ => None,
        }
    }

    fn config_value(self) -> &'static str {
        match self {
            Self::Positions => "Positions",
            Self::OpenOrders => "OpenOrders",
            Self::Balances => "Balances",
            Self::TradeHistory => "TradeHistory",
            Self::FundingHistory => "FundingHistory",
        }
    }
}

impl<'de> Deserialize<'de> for BottomTabConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_config_value(&value).unwrap_or_else(|| {
            let default = Self::default();
            crate::config::push_config_warning(format!(
                "Unknown bottom tab {value:?} in config; using {}",
                default.config_value()
            ));
            default
        }))
    }
}

/// Persisted pane content kind.
#[derive(Debug, Clone, PartialEq)]
pub enum PaneKindConfig {
    AccountSummary,
    Chart {
        chart_id: u64,
    },
    OrderBook {
        id: u64,
    },
    Watchlist,
    LiveWatchlist {
        id: u64,
    },
    PositioningInfo {
        id: u64,
    },
    SessionData {
        id: u64,
    },
    XFeed {
        id: u64,
    },

    Portfolio,
    Income,
    BottomTabs {
        active_tab: BottomTabConfig,
    },
    OrderEntry,
    AdvancedOrders,
    SpaghettiChart {
        spaghetti_id: u64,
    },
    Settings,
    Calendar,
    Liquidations,
    LiquidationsDistribution,
    TrackedTrades,
    TelegramFeed,
    Outcomes,
    HypeEtfs,
    HypeUnstakingQueue,
    /// Legacy or unknown persisted panes that no longer have runtime support.
    Unsupported,
    /// Raw forward-compatible pane data from a newer config schema.
    Unknown(serde_json::Value),
}

/// Stable identity for widget-level appearance overrides.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WidgetPaddingTargetConfig {
    Chart { chart_id: u64 },
    OrderBook { id: u64 },
    Watchlist,
    LiveWatchlist { id: u64 },
    PositioningInfo { id: u64 },
    SessionData { id: u64 },
    XFeed { id: u64 },

    Portfolio,
    Income,
    BottomTabs,
    OrderEntry,
    AdvancedOrders,
    SpaghettiChart { spaghetti_id: u64 },
    Settings,
    Calendar,
    Liquidations,
    LiquidationsDistribution,
    TrackedTrades,
    TelegramFeed,
    Outcomes,
    HypeEtfs,
    HypeUnstakingQueue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WidgetPaddingOverrideConfig {
    pub target: WidgetPaddingTargetConfig,
    pub padding_px: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WidgetPaddingConfig {
    #[serde(default = "default_widget_padding")]
    pub default_px: f32,
    #[serde(
        default,
        deserialize_with = "widget_padding_wire::deserialize_overrides"
    )]
    pub overrides: Vec<WidgetPaddingOverrideConfig>,
}

impl Default for WidgetPaddingConfig {
    fn default() -> Self {
        Self {
            default_px: default_widget_padding(),
            overrides: Vec::new(),
        }
    }
}

impl WidgetPaddingConfig {
    pub fn normalized(self) -> Self {
        let default_px = normalize_widget_padding(self.default_px);
        let mut overrides = BTreeMap::new();

        for item in self.overrides {
            let padding_px = normalize_widget_padding(item.padding_px);
            overrides.insert(item.target, padding_px);
        }

        Self {
            default_px,
            overrides: overrides
                .into_iter()
                .filter(|(_, padding_px)| (*padding_px - default_px).abs() > f32::EPSILON)
                .map(|(target, padding_px)| WidgetPaddingOverrideConfig { target, padding_px })
                .collect(),
        }
    }
}

/// Persisted pane-grid tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaneLayoutConfig {
    Leaf(PaneKindConfig),
    Split {
        axis: AxisConfig,
        ratio: f32,
        a: Box<PaneLayoutConfig>,
        b: Box<PaneLayoutConfig>,
    },
}

impl PaneLayoutConfig {
    pub fn normalize_split_ratios(&mut self) {
        match self {
            PaneLayoutConfig::Leaf(_) => {}
            PaneLayoutConfig::Split { ratio, a, b, .. } => {
                *ratio = normalize_pane_split_ratio(*ratio);
                a.normalize_split_ratios();
                b.normalize_split_ratios();
            }
        }
    }
}

/// Drop panes this version cannot instantiate when building a live runtime layout.
pub fn prune_unsupported_pane_layout(layout: PaneLayoutConfig) -> Option<PaneLayoutConfig> {
    prune_pane_layout(layout, |kind| {
        matches!(
            kind,
            PaneKindConfig::Unsupported
                | PaneKindConfig::AccountSummary
                | PaneKindConfig::Unknown(_)
        )
    })
}

/// Drop known removed panes while preserving raw future pane data for persistence.
pub fn prune_legacy_unsupported_pane_layout(layout: PaneLayoutConfig) -> Option<PaneLayoutConfig> {
    prune_pane_layout(layout, |kind| {
        matches!(
            kind,
            PaneKindConfig::Unsupported | PaneKindConfig::AccountSummary
        )
    })
}

fn prune_pane_layout(
    layout: PaneLayoutConfig,
    should_prune_leaf: fn(&PaneKindConfig) -> bool,
) -> Option<PaneLayoutConfig> {
    match layout {
        PaneLayoutConfig::Leaf(kind) if should_prune_leaf(&kind) => None,
        PaneLayoutConfig::Leaf(kind) => Some(PaneLayoutConfig::Leaf(kind)),
        PaneLayoutConfig::Split { axis, ratio, a, b } => {
            match (
                prune_pane_layout(*a, should_prune_leaf),
                prune_pane_layout(*b, should_prune_leaf),
            ) {
                (Some(a), Some(b)) => Some(PaneLayoutConfig::Split {
                    axis,
                    ratio,
                    a: Box::new(a),
                    b: Box::new(b),
                }),
                (Some(remaining), None) | (None, Some(remaining)) => Some(remaining),
                (None, None) => None,
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedLayout {
    pub name: String,
    #[serde(default)]
    pub pane_layout: Option<PaneLayoutConfig>,
    #[serde(default)]
    pub layout_ratios: Vec<f32>,
    #[serde(default)]
    pub charts: Vec<ChartConfig>,
    #[serde(default)]
    pub order_books: Vec<OrderBookConfig>,
    #[serde(default)]
    pub live_watchlists: Vec<LiveWatchlistConfig>,
    #[serde(default)]
    pub positioning_infos: Vec<PositioningInfoConfig>,
    #[serde(default)]
    pub session_data: Vec<super::SessionDataConfig>,
    #[serde(default)]
    pub x_feeds: Vec<super::XFeedConfig>,
    #[serde(default)]
    pub spaghetti_charts: Vec<SpaghettiChartConfig>,
    #[serde(default)]
    pub widget_padding: WidgetPaddingConfig,

    #[serde(default = "default_symbol")]
    pub active_symbol: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub liquidation_distribution_symbol: Option<String>,
    #[serde(default = "default_timeframe")]
    pub active_timeframe: String,
    #[serde(default = "default_order_kind")]
    pub order_kind: String,
    #[serde(default)]
    pub reduce_only: bool,
    #[serde(default = "super::default_tick_size")]
    pub book_tick_size: f64,
    #[serde(default)]
    pub favourite_symbols: Vec<String>,
    #[serde(default)]
    pub ticker_tape_enabled: bool,

    #[serde(default = "super::default_theme")]
    pub active_theme: String,
    #[serde(default = "default_custom_themes")]
    pub custom_themes: Vec<CustomThemeConfig>,
    #[serde(default)]
    pub sound_enabled: bool,
    #[serde(default)]
    pub desktop_notifications: bool,
    #[serde(default)]
    pub income_alerts_enabled: bool,
    #[serde(default)]
    pub liquidation_alerts_enabled: bool,
    #[serde(default = "super::default_liquidation_alert_threshold")]
    pub liquidation_alert_threshold: f64,
    #[serde(default = "super::default_market_slippage_pct")]
    pub market_slippage_pct: f64,
    #[serde(default)]
    pub tracked_trade_alerts_enabled: bool,
    #[serde(default)]
    pub tracked_trade_aggregation_enabled: bool,
    #[serde(default)]
    pub liquidation_feed_aggregation_enabled: bool,
    #[serde(default = "default_true")]
    pub preset_is_usd: bool,
    #[serde(default)]
    pub order_presets: OrderPresetsConfig,
}

impl fmt::Debug for SavedLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SavedLayout")
            .field("name", &format_args!("<redacted>"))
            .field("pane_layout_present", &self.pane_layout.is_some())
            .field("layout_ratios_len", &self.layout_ratios.len())
            .field("charts_len", &self.charts.len())
            .field("order_books_len", &self.order_books.len())
            .field("live_watchlists_len", &self.live_watchlists.len())
            .field("positioning_infos_len", &self.positioning_infos.len())
            .field("session_data_len", &self.session_data.len())
            .field("x_feeds_len", &self.x_feeds.len())
            .field("spaghetti_charts_len", &self.spaghetti_charts.len())
            .field("favourite_symbols_len", &self.favourite_symbols.len())
            .field("custom_themes_len", &self.custom_themes.len())
            .field("order_presets", &self.order_presets)
            .finish()
    }
}

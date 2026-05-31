mod pane_kind_wire;

use serde::{Deserialize, Serialize};

use super::{
    ChartConfig, CustomThemeConfig, LiveWatchlistConfig, OrderBookConfig, OrderPresetsConfig,
    PositioningInfoConfig, SpaghettiChartConfig, default_custom_themes, default_order_kind,
    default_symbol, default_timeframe,
};

/// Persisted axis for a pane split.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AxisConfig {
    Horizontal,
    Vertical,
}

/// Persisted bottom tab selection.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum BottomTabConfig {
    Positions,
    OpenOrders,
    Balances,
    TradeHistory,
    FundingHistory,
}

/// Persisted pane content kind.
#[derive(Debug, Clone, Serialize, PartialEq)]
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

pub fn prune_unsupported_pane_layout(layout: PaneLayoutConfig) -> Option<PaneLayoutConfig> {
    match layout {
        PaneLayoutConfig::Leaf(PaneKindConfig::Unsupported | PaneKindConfig::AccountSummary) => {
            None
        }
        PaneLayoutConfig::Leaf(kind) => Some(PaneLayoutConfig::Leaf(kind)),
        PaneLayoutConfig::Split { axis, ratio, a, b } => {
            match (
                prune_unsupported_pane_layout(*a),
                prune_unsupported_pane_layout(*b),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    pub spaghetti_charts: Vec<SpaghettiChartConfig>,

    #[serde(default = "default_symbol")]
    pub active_symbol: String,
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

fn default_true() -> bool {
    true
}

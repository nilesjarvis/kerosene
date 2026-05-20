use super::hotkeys::HotkeyConfig;
use super::layouts::{PaneLayoutConfig, SavedLayout};
use super::live_watchlist::LiveWatchlistConfig;
use super::order_presets::OrderPresetsConfig;
use super::panes::{
    ChartConfig, DetachedChartWindowConfig, OrderBookConfig, PositioningInfoConfig,
    SpaghettiChartConfig,
};
use super::screenshot::ChartScreenshotSettingsConfig;
use super::secrets::EncryptedSecretsConfig;
use super::themes::{CustomThemeConfig, default_custom_themes, default_theme};
use super::wallets::{AddressBookEntryConfig, WalletTrackerConfig};
use crate::advanced_order_history::AdvancedOrderHistoryEntry;
use crate::journal::JournalNote;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use zeroize::Zeroizing;

mod accounts;
mod defaults;

pub use accounts::{AccountProfile, CredentialStorageMode};
#[cfg(test)]
pub use defaults::MAX_MARKET_SLIPPAGE_PCT;
use defaults::default_true;
pub use defaults::{
    DEFAULT_MARKET_SLIPPAGE_PCT, DEFAULT_UI_SCALE, MAX_PANE_BORDER_THICKNESS,
    MAX_PANE_CORNER_RADIUS, MAX_UI_SCALE, MIN_PANE_BORDER_THICKNESS, MIN_PANE_CORNER_RADIUS,
    MIN_UI_SCALE, default_layout_ratios, default_liquidation_alert_threshold,
    default_market_slippage_pct, default_order_kind, default_pane_border_thickness,
    default_pane_corner_radius, default_symbol, default_symbol_search_sort_mode, default_tick_size,
    default_timeframe, default_ui_scale, new_secret_id, normalize_market_slippage_pct,
    normalize_pane_border_thickness, normalize_pane_corner_radius, normalize_ui_scale,
};

// ---------------------------------------------------------------------------
// Config Schema
// ---------------------------------------------------------------------------

/// Global market universe shown by the terminal.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum MarketUniverseConfig {
    #[default]
    All,
    Hip3Dex {
        dex: String,
    },
}

impl MarketUniverseConfig {
    pub fn hip3_dex(dex: impl Into<String>) -> Self {
        Self::Hip3Dex { dex: dex.into() }.normalized()
    }

    pub fn normalized(self) -> Self {
        match self {
            Self::All => Self::All,
            Self::Hip3Dex { dex } => {
                let dex = dex.trim().to_ascii_lowercase();
                if dex.is_empty() {
                    Self::All
                } else {
                    Self::Hip3Dex { dex }
                }
            }
        }
    }

    pub fn selected_hip3_dex(&self) -> Option<&str> {
        match self {
            Self::All => None,
            Self::Hip3Dex { dex } => Some(dex.as_str()),
        }
    }
}

impl fmt::Display for MarketUniverseConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => f.write_str("All Markets"),
            Self::Hip3Dex { dex } => write!(f, "HIP-3: {dex}"),
        }
    }
}

/// Persisted application config. Saved as JSON to the platform config directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeroseneConfig {
    #[serde(default)]
    pub saved_layouts: Vec<SavedLayout>,
    #[serde(default)]
    pub active_layout_name: Option<String>,
    #[serde(default)]
    pub credential_storage_mode: CredentialStorageMode,
    #[serde(default)]
    pub encrypted_secrets: Option<EncryptedSecretsConfig>,
    #[serde(default)]
    pub main_window_width: Option<f32>,
    #[serde(default)]
    pub main_window_height: Option<f32>,
    #[serde(default)]
    pub main_window_x: Option<f32>,
    #[serde(default)]
    pub main_window_y: Option<f32>,
    #[serde(default)]
    pub accounts: Vec<AccountProfile>,
    #[serde(default)]
    pub active_account_index: usize,

    /// Legacy Agent private key (hex).
    #[serde(default)]
    #[serde(skip_serializing)]
    pub agent_key: Zeroizing<String>,
    /// Legacy Connected wallet address.
    #[serde(default)]
    pub wallet_address: String,
    /// Active trading symbol (e.g. "HYPE", "BTC", "xyz:NVDA").
    /// Still used as the symbol for order entry / order book.
    #[serde(default = "default_symbol")]
    pub active_symbol: String,
    /// Active chart timeframe (legacy -- used when `charts` is empty).
    #[serde(default = "default_timeframe")]
    pub active_timeframe: String,
    /// Default order kind.
    #[serde(default = "default_order_kind")]
    pub order_kind: String,
    /// Whether reduce-only is toggled on by default.
    #[serde(default)]
    pub reduce_only: bool,
    /// Whether order quantity inputs default to USD notional instead of coin size.
    #[serde(default)]
    pub order_quantity_is_usd: bool,
    /// Global UI scale multiplier. Values below 1.0 make the terminal denser.
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f32,
    /// Width of the divider between pane widgets in pixels.
    #[serde(default = "default_pane_border_thickness")]
    pub pane_border_thickness: f32,
    /// Corner radius applied to pane widgets in pixels.
    #[serde(default = "default_pane_corner_radius")]
    pub pane_corner_radius: f32,
    /// Order book tick size.
    #[serde(default = "default_tick_size")]
    pub book_tick_size: f64,
    /// Selected sort mode for the global symbol search.
    #[serde(default = "default_symbol_search_sort_mode")]
    pub symbol_search_sort_mode: String,
    /// Global visible market universe.
    #[serde(default)]
    pub market_universe: MarketUniverseConfig,
    /// Global settings used for chart screenshots.
    #[serde(default)]
    pub chart_screenshot_settings: ChartScreenshotSettingsConfig,
    /// Pane layout split ratios (top-to-bottom, left-to-right order).
    /// The 5 ratios correspond to the splits in the pane tree:
    /// [account_bar, main_vs_bottom, chart_vs_right, orderbook_vs_watchlist, tabs_vs_entry]
    #[serde(default = "default_layout_ratios")]
    pub layout_ratios: Vec<f32>,
    /// Full pane layout tree (widget placement + split ratios).
    #[serde(default)]
    pub pane_layout: Option<PaneLayoutConfig>,
    /// Per-chart pane configurations. Empty = legacy single-chart.
    #[serde(default)]
    pub charts: Vec<ChartConfig>,
    /// Detached candlestick chart windows to reopen on startup.
    #[serde(default)]
    pub detached_chart_windows: Vec<DetachedChartWindowConfig>,
    #[serde(default)]
    pub order_books: Vec<OrderBookConfig>,
    /// Favourite symbol keys (e.g. ["HYPE", "BTC", "@107"]).
    #[serde(default)]
    pub live_watchlists: Vec<LiveWatchlistConfig>,
    #[serde(default)]
    pub positioning_infos: Vec<PositioningInfoConfig>,

    pub favourite_symbols: Vec<String>,
    /// Globally hidden ticker symbols. Matching is intentionally broad for plain
    /// tickers, so muting BTC also hides UBTC and HIP-3 BTC variants.
    #[serde(default)]
    pub muted_tickers: Vec<String>,
    /// Hydromancer API key for liquidation and tracked-trade streams.
    #[serde(default)]
    #[serde(skip_serializing)]
    pub hydromancer_api_key: Zeroizing<String>,
    /// HyperDash API key for liquidation heatmap data.
    #[serde(default)]
    #[serde(skip_serializing)]
    pub hyperdash_api_key: Zeroizing<String>,
    /// Sound notifications enabled.
    #[serde(default)]
    pub sound_enabled: bool,
    /// Desktop notifications enabled.
    #[serde(default)]
    pub desktop_notifications: bool,
    /// Hourly positive-interest alerts enabled.
    #[serde(default)]
    pub income_alerts_enabled: bool,
    /// Hide dollar-denominated PnL values in account views.
    #[serde(default)]
    pub hide_pnl: bool,
    /// Account-scoped position symbols hidden from the positions widget.
    #[serde(default)]
    pub hidden_positions_by_account: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub liquidation_alerts_enabled: bool,
    #[serde(default = "default_liquidation_alert_threshold")]
    pub liquidation_alert_threshold: f64,
    /// Percent slippage used to derive IOC market-order limit prices.
    #[serde(default = "default_market_slippage_pct")]
    pub market_slippage_pct: f64,
    #[serde(default)]
    pub tracked_trade_alerts_enabled: bool,
    #[serde(default)]
    pub tracked_trade_aggregation_enabled: bool,
    #[serde(default)]
    pub liquidation_feed_aggregation_enabled: bool,
    /// Per-spaghetti (comparison) chart configurations.
    #[serde(default)]
    pub spaghetti_charts: Vec<SpaghettiChartConfig>,
    /// Wallet tracker window state.
    #[serde(default)]
    pub wallet_tracker: WalletTrackerConfig,
    /// Shared wallet labels and display metadata.
    #[serde(default)]
    pub address_book: Vec<AddressBookEntryConfig>,
    /// Active theme string (e.g. "Dark", "Catppuccin Mocha", "Custom: E-Ink")
    #[serde(default = "default_theme")]
    pub active_theme: String,
    /// User-created custom themes.
    #[serde(default = "default_custom_themes")]
    pub custom_themes: Vec<CustomThemeConfig>,
    /// Trading Journal notes, mapped by Trade ID (e.g. "{coin}_{start_time_ms}").
    #[serde(default)]
    pub journal_entries: HashMap<String, JournalNote>,
    /// Trading Journal notes grouped by persisted account secret ID.
    #[serde(default)]
    pub journal_entries_by_account: HashMap<String, HashMap<String, JournalNote>>,
    /// User-configured quick-order presets.
    #[serde(default)]
    pub order_presets: OrderPresetsConfig,
    /// Completed/stopped advanced order history. Live advanced orders are not
    /// persisted or resumed after restart.
    #[serde(default)]
    pub advanced_order_history: Vec<AdvancedOrderHistoryEntry>,
    /// Whether presets are currently displaying USD or COIN.
    #[serde(default = "default_true")]
    pub preset_is_usd: bool,
    /// Global application hotkeys
    #[serde(default)]
    pub hotkeys: Vec<HotkeyConfig>,
}

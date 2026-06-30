use super::hotkeys::{HotkeyConfig, HotkeyPrefixConfig};
use super::layouts::{PaneLayoutConfig, SavedLayout, WidgetPaddingConfig};
use super::live_watchlist::LiveWatchlistConfig;
use super::order_presets::OrderPresetsConfig;
use super::panes::{
    ChartConfig, DetachedChartWindowConfig, OrderBookConfig, PositioningInfoConfig,
    SessionDataConfig, SpaghettiChartConfig, XFeedConfig,
};
use super::screenshot::ChartScreenshotSettingsConfig;
use super::secrets::EncryptedSecretsConfig;
use super::themes::{CustomThemeConfig, default_custom_themes, default_theme};
use super::wallets::{AddressBookEntryConfig, WalletClustersConfig, WalletTrackerConfig};
use super::{CustomFontConfig, DisplayFontConfig};
use crate::advanced_order_history::AdvancedOrderHistoryEntry;
use crate::journal::JournalNote;
use crate::telegram_feed::{TelegramFeedPrivateChannelConfig, default_telegram_feed_channels};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use zeroize::Zeroizing;

mod accounts;
mod candles;
mod crosshair;
mod defaults;
mod denomination;
mod market_universe;
mod toast;

pub use accounts::{AccountProfile, CredentialStorageMode};
pub use candles::{ChartBackfillSource, ChartHollowCandleMode, ChartSeriesStyle, ReadDataProvider};
pub use crosshair::{
    ChartCrosshairStyle, ChartHudOrderSound, ChartHudReadoutConfig, ChartHudReadoutElement,
};
#[cfg(test)]
pub use defaults::MAX_MARKET_SLIPPAGE_PCT;
pub(crate) use defaults::default_true;
pub use defaults::{
    DEFAULT_CHART_CHROMATIC_ABERRATION_STRENGTH, DEFAULT_CHART_CROSSHAIR_SCALE,
    DEFAULT_CHART_DOTTED_BACKGROUND_OPACITY, DEFAULT_CHART_EDGE_BLUR_STRENGTH,
    DEFAULT_CHART_FISHEYE_STRENGTH, DEFAULT_CHART_HUD_ORDER_SOUND_VOLUME,
    DEFAULT_MARKET_SLIPPAGE_PCT, DEFAULT_UI_SCALE, MAX_ALFRED_POPUP_SCALE,
    MAX_CHART_CHROMATIC_ABERRATION_STRENGTH, MAX_CHART_CROSSHAIR_SCALE,
    MAX_CHART_DOTTED_BACKGROUND_OPACITY, MAX_CHART_EDGE_BLUR_STRENGTH, MAX_CHART_FISHEYE_STRENGTH,
    MAX_CHART_HUD_ORDER_SOUND_VOLUME, MAX_PANE_BORDER_THICKNESS, MAX_PANE_CORNER_RADIUS,
    MAX_UI_SCALE, MAX_WIDGET_PADDING, MIN_ALFRED_POPUP_SCALE,
    MIN_CHART_CHROMATIC_ABERRATION_STRENGTH, MIN_CHART_CROSSHAIR_SCALE,
    MIN_CHART_DOTTED_BACKGROUND_OPACITY, MIN_CHART_EDGE_BLUR_STRENGTH, MIN_CHART_FISHEYE_STRENGTH,
    MIN_CHART_HUD_ORDER_SOUND_VOLUME, MIN_PANE_BORDER_THICKNESS, MIN_PANE_CORNER_RADIUS,
    MIN_UI_SCALE, MIN_WIDGET_PADDING, default_alfred_popup_scale,
    default_chart_chromatic_aberration_strength, default_chart_crosshair_scale,
    default_chart_dotted_background_opacity, default_chart_edge_blur_strength,
    default_chart_fisheye_strength, default_chart_hud_order_sound_volume, default_layout_ratios,
    default_liquidation_alert_threshold, default_market_slippage_pct, default_order_kind,
    default_pane_border_thickness, default_pane_corner_radius, default_symbol,
    default_symbol_search_sort_mode, default_tick_size, default_timeframe, default_ui_scale,
    default_widget_padding, effective_radius, new_secret_id, normalize_alfred_popup_scale,
    normalize_chart_chromatic_aberration_strength, normalize_chart_crosshair_scale,
    normalize_chart_dotted_background_opacity, normalize_chart_edge_blur_strength,
    normalize_chart_fisheye_strength, normalize_chart_hud_order_sound_volume,
    normalize_market_slippage_pct, normalize_pane_border_thickness, normalize_pane_corner_radius,
    normalize_pane_split_ratio, normalize_ui_scale, normalize_widget_padding,
};
pub use denomination::DisplayDenominationConfig;
pub use market_universe::MarketUniverseConfig;
pub use toast::ToastPosition;

// ---------------------------------------------------------------------------
// Config Schema
// ---------------------------------------------------------------------------

/// Persisted application config. Saved as JSON to the platform config directory.
#[derive(Clone, Serialize, Deserialize)]
pub struct KeroseneConfig {
    #[serde(default)]
    pub saved_layouts: Vec<SavedLayout>,
    #[serde(default)]
    pub active_layout_name: Option<String>,
    /// Whether the first-run application welcome screen has been dismissed.
    #[serde(default = "default_legacy_app_onboarding_dismissed")]
    pub app_onboarding_dismissed: bool,
    #[serde(default)]
    pub credential_storage_mode: CredentialStorageMode,
    #[serde(default)]
    pub encrypted_secrets: Option<EncryptedSecretsConfig>,
    #[serde(skip)]
    pub secret_migration_save_blocked: bool,
    #[serde(default)]
    pub main_window_width: Option<f32>,
    #[serde(default)]
    pub main_window_height: Option<f32>,
    #[serde(default)]
    pub main_window_x: Option<f32>,
    #[serde(default)]
    pub main_window_y: Option<f32>,
    /// Last trading journal window width in logical pixels.
    #[serde(default)]
    pub journal_window_width: Option<f32>,
    /// Last trading journal window height in logical pixels.
    #[serde(default)]
    pub journal_window_height: Option<f32>,
    #[serde(default)]
    pub accounts: Vec<AccountProfile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_keychain_profile_deletions: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub pending_keychain_cleanup_all: bool,
    #[serde(skip)]
    pub secret_cleanup_state_dirty: bool,
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
    /// Use dotted chart plot backgrounds instead of standard grid lines.
    #[serde(default)]
    pub chart_dotted_background: bool,
    /// Opacity for dotted chart plot backgrounds.
    #[serde(default = "default_chart_dotted_background_opacity")]
    pub chart_dotted_background_opacity: f32,
    /// Apply a theme-aware gradient to chart plot backgrounds.
    #[serde(default)]
    pub chart_gradient_background: bool,
    /// Legacy toggle for hollow bullish candle bodies.
    #[serde(default)]
    #[serde(skip_serializing)]
    pub chart_hollow_candles: bool,
    /// Which candle directions should render hollow instead of solid fills.
    #[serde(default)]
    pub chart_hollow_candle_mode: ChartHollowCandleMode,
    /// Whether the main price series renders as candlesticks or a line + area fill.
    #[serde(default)]
    pub chart_series_style: ChartSeriesStyle,
    /// Apply a subtle barrel lens projection to candlestick chart canvases.
    #[serde(default)]
    pub chart_fisheye_enabled: bool,
    /// Strength of the chart fisheye projection.
    #[serde(default = "default_chart_fisheye_strength")]
    pub chart_fisheye_strength: f32,
    /// Apply subtle red/cyan channel separation to chart canvas geometry.
    #[serde(default)]
    pub chart_chromatic_aberration_enabled: bool,
    /// Strength of the chart chromatic aberration effect.
    #[serde(default = "default_chart_chromatic_aberration_strength")]
    pub chart_chromatic_aberration_strength: f32,
    /// Apply a subtle edge blur to chart canvas geometry.
    #[serde(default)]
    pub chart_edge_blur_enabled: bool,
    /// Strength of the chart edge blur effect.
    #[serde(default = "default_chart_edge_blur_strength")]
    pub chart_edge_blur_strength: f32,
    /// Crosshair or gaming HUD style used by chart canvases.
    #[serde(default)]
    pub chart_crosshair_style: ChartCrosshairStyle,
    /// Whether chart crosshairs draw full-width/full-height guide lines.
    #[serde(default = "default_true")]
    pub chart_crosshair_guides_enabled: bool,
    /// User-controlled multiplier for local chart crosshair or HUD size.
    #[serde(default = "default_chart_crosshair_scale")]
    pub chart_crosshair_scale: f32,
    /// Sound effect played when HUD chart trading submits an order.
    #[serde(default)]
    pub chart_hud_order_sound: ChartHudOrderSound,
    /// Imported WAV file name for the HUD chart order sound, stored in the config directory.
    #[serde(default)]
    pub chart_hud_order_sound_file: Option<String>,
    /// Volume multiplier for the HUD chart order sound.
    #[serde(default = "default_chart_hud_order_sound_volume")]
    pub chart_hud_order_sound_volume: f32,
    /// Whether HUD game-mode control changes (mode/side/arm/size) play interface clicks.
    #[serde(default = "default_true")]
    pub chart_hud_ui_sounds: bool,
    /// HUD chart readout rows displayed around the central order type and size.
    #[serde(default)]
    pub chart_hud_readout: ChartHudReadoutConfig,
    /// User-controlled scale for the Alfred command popup.
    #[serde(default = "default_alfred_popup_scale")]
    pub alfred_popup_scale: f32,
    /// Global read-only data provider for supported market/account data.
    #[serde(default)]
    pub read_data_provider: ReadDataProvider,
    /// Legacy REST provider used for historical chart candle backfills.
    #[serde(default)]
    #[serde(skip_serializing)]
    pub chart_backfill_source: ChartBackfillSource,
    /// Global display font used by default UI text.
    #[serde(default)]
    pub display_font: DisplayFontConfig,
    /// Global font used where the terminal explicitly aligns text as monospace.
    #[serde(default)]
    pub monospace_font: DisplayFontConfig,
    /// User-imported fonts available to display and monospace font pickers.
    #[serde(default)]
    pub custom_fonts: Vec<CustomFontConfig>,
    /// Width of the divider between pane widgets in pixels.
    #[serde(default = "default_pane_border_thickness")]
    pub pane_border_thickness: f32,
    /// Corner radius applied to pane widgets in pixels.
    #[serde(default = "default_pane_corner_radius")]
    pub pane_corner_radius: f32,
    /// Whether the main window has an exterior gutter matching pane dividers.
    #[serde(default = "default_true")]
    pub outer_widget_border_enabled: bool,
    /// Padding applied inside pane widgets, with optional per-widget overrides.
    #[serde(default)]
    pub widget_padding: WidgetPaddingConfig,
    /// Whether Kerosene draws its own OS title bar instead of using native window chrome.
    #[serde(default = "default_true")]
    pub custom_window_chrome_enabled: bool,
    /// Order book tick size.
    #[serde(default = "default_tick_size")]
    pub book_tick_size: f64,
    /// Selected sort mode for the global symbol search.
    #[serde(default = "default_symbol_search_sort_mode")]
    pub symbol_search_sort_mode: String,
    /// Global visible market universe.
    #[serde(default)]
    pub market_universe: MarketUniverseConfig,
    /// Independently selected perp market for the liquidation distribution widget.
    #[serde(default)]
    pub liquidation_distribution_symbol: String,
    /// Display-only denomination for USD-valued readouts.
    #[serde(default)]
    pub display_denomination: DisplayDenominationConfig,
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
    #[serde(default)]
    pub session_data: Vec<SessionDataConfig>,
    #[serde(default)]
    pub x_feeds: Vec<XFeedConfig>,

    /// Whether the favourites ticker tape is visible below the account bar.
    #[serde(default)]
    pub ticker_tape_enabled: bool,
    #[serde(default)]
    pub favourite_symbols: Vec<String>,
    /// Globally hidden ticker symbols. Matching is intentionally broad for plain
    /// tickers, so muting BTC also hides UBTC and HIP-3 BTC variants.
    #[serde(default)]
    pub muted_tickers: Vec<String>,
    /// Cached display labels for outcome trade coins ("#NNN" -> label) so fills,
    /// journal entries, and balances on expired HIP-4 markets keep their names.
    #[serde(default)]
    pub outcome_display_labels: HashMap<String, String>,
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
    /// Screen corner where in-app toast notifications stack.
    #[serde(default)]
    pub toast_position: ToastPosition,
    /// Whether toasts slide and fade when entering and leaving.
    #[serde(default = "default_true")]
    pub toast_animations_enabled: bool,
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
    /// Project in-flight order actions into the Orders and Positions tabs
    /// before the exchange confirms them.
    #[serde(default)]
    pub optimistic_account_updates: bool,
    /// Use Hydromancer l2Book ticks to update open-position PnL in real time.
    #[serde(default)]
    pub hydromancer_realtime_position_pnl_enabled: bool,
    #[serde(default)]
    pub tracked_trade_alerts_enabled: bool,
    #[serde(default)]
    pub tracked_trade_aggregation_enabled: bool,
    #[serde(default)]
    pub liquidation_feed_aggregation_enabled: bool,
    #[serde(default)]
    pub telegram_feed_notifications_enabled: bool,
    /// Show outcome (prediction) markets in Telegram Feed ticker chips. Defaults
    /// to true so upgrades keep existing behaviour.
    #[serde(default = "default_true")]
    pub telegram_feed_include_outcome_markets: bool,
    /// Whether the Telegram Feed onboarding (Connect) screen has been dismissed.
    /// Defaults to false so first-run users see it once.
    #[serde(default)]
    pub telegram_feed_onboarding_dismissed: bool,
    /// Optional Telegram MTProto fast-feed mode. Secret session material is stored separately.
    #[serde(default)]
    pub telegram_feed_fast_mode_enabled: bool,
    /// Telegram developer API ID used by optional MTProto fast-feed mode.
    #[serde(default)]
    pub telegram_feed_fast_api_id: Option<i32>,
    /// Public Telegram channel usernames shown by the Telegram Feed widget.
    #[serde(default = "default_telegram_feed_channels")]
    pub telegram_feed_channels: Vec<String>,
    /// Private broadcast Telegram channels selected from the signed-in MTProto account.
    #[serde(default)]
    pub telegram_feed_private_channels: Vec<TelegramFeedPrivateChannelConfig>,
    /// Per-spaghetti (comparison) chart configurations.
    #[serde(default)]
    pub spaghetti_charts: Vec<SpaghettiChartConfig>,
    /// Wallet tracker window state.
    #[serde(default)]
    pub wallet_tracker: WalletTrackerConfig,
    /// Tradable wallet clusters and cluster window state.
    #[serde(default)]
    pub wallet_clusters: WalletClustersConfig,
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
    #[serde(default, deserialize_with = "deserialize_hotkeys")]
    pub hotkeys: Vec<HotkeyConfig>,
    /// Modifier prefix used with number keys to switch the active chart timeframe.
    #[serde(default)]
    pub chart_timeframe_hotkey_prefix: Option<HotkeyPrefixConfig>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn default_legacy_app_onboarding_dismissed() -> bool {
    true
}

fn deserialize_hotkeys<'de, D>(deserializer: D) -> Result<Vec<HotkeyConfig>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    let Some(entries) = value.as_array() else {
        crate::config::push_config_warning(
            "Invalid hotkeys config; using no configured hotkeys".to_string(),
        );
        return Ok(Vec::new());
    };

    Ok(entries
        .iter()
        .filter_map(
            |entry| match serde_json::from_value::<HotkeyConfig>(entry.clone()) {
                Ok(hotkey) => Some(hotkey),
                Err(_) => {
                    crate::config::push_config_warning(
                        "Invalid hotkey entry in config; dropping hotkey".to_string(),
                    );
                    None
                }
            },
        )
        .collect())
}

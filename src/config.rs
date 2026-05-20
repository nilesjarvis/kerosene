use std::sync::{Mutex, OnceLock};

mod clear;
mod files;
mod hotkeys;
mod layouts;
mod live_watchlist;
mod order_presets;
mod panes;
mod schema;
mod screenshot;
mod secrets;
mod themes;
mod wallets;

pub use clear::{ClearConfigSummary, clear_all_configs};
pub use files::{journal_cache_path, load_config, save_config};
pub use hotkeys::{HotkeyAction, HotkeyConfig, HotkeyPrefixConfig};
pub use layouts::{
    AxisConfig, BottomTabConfig, PaneKindConfig, PaneLayoutConfig, SavedLayout,
    prune_unsupported_pane_layout,
};
pub use live_watchlist::{
    LiveWatchlistColumn, LiveWatchlistConfig, LiveWatchlistSortColumn, SortDirection,
    default_live_watchlist_columns,
};
pub use order_presets::{OrderPreset, OrderPresetsConfig};
pub use panes::{
    ChartConfig, DetachedChartWindowConfig, MacroIndicatorsConfig, OrderBookConfig,
    OrderBookDisplayModeConfig, OrderBookSymbolModeConfig, PositioningInfoConfig,
    SpaghettiChartConfig, default_detached_chart_window_height,
    default_detached_chart_window_width,
};
#[cfg(test)]
pub use schema::MAX_MARKET_SLIPPAGE_PCT;
#[cfg(test)]
pub use schema::default_ui_scale;
pub use schema::{
    AccountProfile, CredentialStorageMode, DEFAULT_MARKET_SLIPPAGE_PCT, DEFAULT_UI_SCALE,
    KeroseneConfig, MAX_PANE_BORDER_THICKNESS, MAX_PANE_CORNER_RADIUS, MAX_UI_SCALE,
    MIN_PANE_BORDER_THICKNESS, MIN_PANE_CORNER_RADIUS, MIN_UI_SCALE, MarketUniverseConfig,
    default_layout_ratios, default_liquidation_alert_threshold, default_market_slippage_pct,
    default_order_kind, default_pane_border_thickness, default_pane_corner_radius, default_symbol,
    default_tick_size, default_timeframe, new_secret_id, normalize_market_slippage_pct,
    normalize_pane_border_thickness, normalize_pane_corner_radius, normalize_ui_scale,
};
pub use screenshot::ChartScreenshotSettingsConfig;
pub use secrets::{
    EncryptedSecretsConfig, SecretPayload, clear_global_secrets, clear_profile_secrets,
    decrypt_secrets, encrypt_secrets, store_global_hydromancer_secret,
    store_global_hyperdash_secret, store_profile_secrets, take_secret_warnings,
};
pub(crate) use themes::default_custom_themes;
pub use themes::{CustomThemeConfig, default_theme};
pub use wallets::{
    AddressBookEntryConfig, TrackedWalletConfig, WALLET_LABELS_EXPORT_SCHEMA, WalletLabelsExport,
    WalletTrackerConfig,
};

use files::{backup_config_path, config_path};

fn config_warnings() -> &'static Mutex<Vec<String>> {
    static WARNINGS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    WARNINGS.get_or_init(|| Mutex::new(Vec::new()))
}

pub(super) fn push_config_warning(message: String) {
    if let Ok(mut warnings) = config_warnings().lock() {
        warnings.push(message);
    }
}

pub fn take_config_warnings() -> Vec<String> {
    config_warnings()
        .lock()
        .map(|mut warnings| std::mem::take(&mut *warnings))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;

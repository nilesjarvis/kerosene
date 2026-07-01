#[cfg(test)]
use std::cell::RefCell;
#[cfg(not(test))]
use std::sync::{Mutex, OnceLock};

mod clear;
mod files;
mod fonts;
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
pub(crate) use files::in_memory_config_mode;
#[cfg(test)]
pub(crate) use files::installed_config_save_error_for_test;
pub(crate) use files::set_in_memory_config_mode;
pub use files::{
    api_cache_dir, custom_font_path, custom_sound_path, font_storage_dir, journal_cache_path,
    load_config, save_config, sound_storage_dir,
};
pub(crate) use files::{config_save_installed_snapshot, user_config_dir, user_config_path};
pub(crate) use fonts::{
    BUNDLED_DISPLAY_FONT_FAMILIES, DM_SANS_FONT_FAMILY, INTER_FONT_FAMILY, QUANTICO_FONT_FAMILY,
    ROBOTO_FONT_FAMILY, ROBOTO_MONO_FONT_FAMILY, UBUNTU_SANS_FONT_FAMILY,
    UBUNTU_SANS_MONO_FONT_FAMILY, bundled_display_font_family, normalize_custom_fonts,
    normalize_display_font,
};
pub use fonts::{CustomFontConfig, DisplayFontConfig};
pub use hotkeys::{HotkeyAction, HotkeyConfig, HotkeyPrefixConfig};
pub use layouts::{
    AxisConfig, BottomTabConfig, PaneKindConfig, PaneLayoutConfig, SavedLayout,
    WidgetPaddingConfig, WidgetPaddingOverrideConfig, WidgetPaddingTargetConfig,
    prune_legacy_unsupported_pane_layout, prune_unsupported_pane_layout,
};
pub use live_watchlist::{
    LiveWatchlistColumn, LiveWatchlistConfig, LiveWatchlistSortColumn, SortDirection,
    default_live_watchlist_columns,
};
pub use order_presets::{OrderPreset, OrderPresetsConfig};
pub use panes::{
    ChartConfig, DetachedChartWindowConfig, MacroIndicatorsConfig, OrderBookConfig,
    OrderBookDisplayModeConfig, OrderBookSymbolModeConfig, PositioningInfoConfig,
    SessionDataConfig, SpaghettiChartConfig, XFeedConfig, default_detached_chart_window_height,
    default_detached_chart_window_width,
};
#[cfg(test)]
pub use schema::MAX_MARKET_SLIPPAGE_PCT;
#[cfg(test)]
pub use schema::default_alfred_popup_scale;
pub(crate) use schema::default_true;
#[cfg(test)]
pub use schema::default_ui_scale;
pub use schema::{
    AccountProfile, ChartBackfillSource, ChartCrosshairStyle, ChartHollowCandleMode,
    ChartHudOrderSound, ChartHudReadoutConfig, ChartHudReadoutElement, ChartSeriesStyle,
    CredentialStorageMode, DEFAULT_CHART_CHROMATIC_ABERRATION_STRENGTH,
    DEFAULT_CHART_CROSSHAIR_SCALE, DEFAULT_CHART_DOTTED_BACKGROUND_OPACITY,
    DEFAULT_CHART_EDGE_BLUR_STRENGTH, DEFAULT_CHART_FISHEYE_STRENGTH,
    DEFAULT_CHART_HUD_ORDER_SOUND_VOLUME, DEFAULT_MARKET_SLIPPAGE_PCT, DEFAULT_UI_SCALE,
    DisplayDenominationConfig, KeroseneConfig, MAX_ALFRED_POPUP_SCALE,
    MAX_CHART_CHROMATIC_ABERRATION_STRENGTH, MAX_CHART_CROSSHAIR_SCALE,
    MAX_CHART_DOTTED_BACKGROUND_OPACITY, MAX_CHART_EDGE_BLUR_STRENGTH, MAX_CHART_FISHEYE_STRENGTH,
    MAX_CHART_HUD_ORDER_SOUND_VOLUME, MAX_PANE_BORDER_THICKNESS, MAX_PANE_CORNER_RADIUS,
    MAX_UI_SCALE, MAX_WIDGET_PADDING, MIN_ALFRED_POPUP_SCALE,
    MIN_CHART_CHROMATIC_ABERRATION_STRENGTH, MIN_CHART_CROSSHAIR_SCALE,
    MIN_CHART_DOTTED_BACKGROUND_OPACITY, MIN_CHART_EDGE_BLUR_STRENGTH, MIN_CHART_FISHEYE_STRENGTH,
    MIN_CHART_HUD_ORDER_SOUND_VOLUME, MIN_PANE_BORDER_THICKNESS, MIN_PANE_CORNER_RADIUS,
    MIN_UI_SCALE, MIN_WIDGET_PADDING, MarketUniverseConfig, ReadDataProvider, ToastPosition,
    default_chart_chromatic_aberration_strength, default_chart_crosshair_scale,
    default_chart_dotted_background_opacity, default_chart_edge_blur_strength,
    default_chart_fisheye_strength, default_layout_ratios, default_liquidation_alert_threshold,
    default_market_slippage_pct, default_order_kind, default_pane_border_thickness,
    default_pane_corner_radius, default_symbol, default_tick_size, default_timeframe,
    default_widget_padding, effective_radius, new_secret_id, normalize_alfred_popup_scale,
    normalize_chart_chromatic_aberration_strength, normalize_chart_crosshair_scale,
    normalize_chart_dotted_background_opacity, normalize_chart_edge_blur_strength,
    normalize_chart_fisheye_strength, normalize_chart_hud_order_sound_volume,
    normalize_market_slippage_pct, normalize_pane_border_thickness, normalize_pane_corner_radius,
    normalize_pane_split_ratio, normalize_ui_scale, normalize_widget_padding,
};
pub use screenshot::ChartScreenshotSettingsConfig;
pub(crate) use secrets::push_secret_warning;
pub use secrets::{
    EncryptedSecretsConfig, SecretPayload, clear_all_keychain_secrets,
    clear_keychain_secret_payload, clear_profile_secrets, decrypt_secrets, encrypt_secrets,
    store_keychain_secrets_with_profile_removals_with_integrations, take_secret_warnings,
};
pub(crate) use secrets::{
    clear_profile_secrets_by_id, load_global_secrets as load_legacy_global_secrets,
    load_profile_secrets as load_legacy_profile_secrets,
};
pub(crate) use secrets::{load_keychain_secret_payload, store_secret_payload};
pub(crate) use themes::default_custom_themes;
pub use themes::{CustomThemeConfig, default_theme};
pub use wallets::{
    AddressBookEntryConfig, TrackedWalletConfig, WALLET_LABELS_EXPORT_SCHEMA, WalletClusterConfig,
    WalletClusterMemberConfig, WalletClustersConfig, WalletLabelsExport, WalletTrackerConfig,
    default_wallet_cluster_member_weight, default_wallet_clusters_height,
    default_wallet_clusters_width,
};

use files::{backup_config_path, config_path, config_sidecar_prefix};

#[cfg(not(test))]
fn config_warnings() -> &'static Mutex<Vec<String>> {
    static WARNINGS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    WARNINGS.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(test)]
thread_local! {
    static CONFIG_WARNINGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

#[cfg(not(test))]
pub(super) fn push_config_warning(message: String) {
    if let Ok(mut warnings) = config_warnings().lock() {
        warnings.push(message);
    }
}

#[cfg(test)]
pub(super) fn push_config_warning(message: String) {
    CONFIG_WARNINGS.with(|warnings| warnings.borrow_mut().push(message));
}

#[cfg(not(test))]
pub fn take_config_warnings() -> Vec<String> {
    config_warnings()
        .lock()
        .map(|mut warnings| std::mem::take(&mut *warnings))
        .unwrap_or_default()
}

#[cfg(test)]
pub fn take_config_warnings() -> Vec<String> {
    CONFIG_WARNINGS.with(|warnings| std::mem::take(&mut *warnings.borrow_mut()))
}

#[cfg(test)]
mod tests;

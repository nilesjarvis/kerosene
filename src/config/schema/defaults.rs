use super::{CredentialStorageMode, KeroseneConfig};
use crate::config::wallets::{default_wallet_tracker_height, default_wallet_tracker_width};
use crate::config::{
    OrderPresetsConfig, WalletTrackerConfig, default_custom_themes, default_theme,
};
use crate::pane_state::{
    DEFAULT_PANE_BORDER_THICKNESS, DEFAULT_PANE_CORNER_RADIUS, DEFAULT_WIDGET_PADDING,
};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Config Defaults
// ---------------------------------------------------------------------------

pub fn default_symbol() -> String {
    "HYPE".to_string()
}

pub(crate) fn default_true() -> bool {
    true
}

pub fn default_timeframe() -> String {
    "H1".to_string()
}

pub fn default_order_kind() -> String {
    "Limit".to_string()
}

pub fn default_tick_size() -> f64 {
    0.01
}

pub fn default_symbol_search_sort_mode() -> String {
    "relevance".to_string()
}

pub fn default_layout_ratios() -> Vec<f32> {
    vec![0.70, 0.50, 0.55, 0.65]
}

pub fn new_secret_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("acct-{}-{}", std::process::id(), nanos)
}

pub fn default_liquidation_alert_threshold() -> f64 {
    10000.0
}

pub const DEFAULT_MARKET_SLIPPAGE_PCT: f64 = 1.0;
pub const MAX_MARKET_SLIPPAGE_PCT: f64 = 20.0;

pub fn default_market_slippage_pct() -> f64 {
    DEFAULT_MARKET_SLIPPAGE_PCT
}

pub fn normalize_market_slippage_pct(value: f64) -> Option<f64> {
    (value.is_finite() && (0.0..=MAX_MARKET_SLIPPAGE_PCT).contains(&value)).then_some(value)
}

pub const DEFAULT_UI_SCALE: f32 = 1.0;
pub const MIN_UI_SCALE: f32 = 0.75;
pub const MAX_UI_SCALE: f32 = 1.10;
pub const DEFAULT_ALFRED_POPUP_SCALE: f32 = 1.0;
pub const MIN_ALFRED_POPUP_SCALE: f32 = 0.85;
pub const MAX_ALFRED_POPUP_SCALE: f32 = 1.60;
pub const DEFAULT_CHART_DOTTED_BACKGROUND_OPACITY: f32 = 0.14;
pub const MIN_CHART_DOTTED_BACKGROUND_OPACITY: f32 = 0.04;
pub const MAX_CHART_DOTTED_BACKGROUND_OPACITY: f32 = 0.35;
pub const DEFAULT_CHART_FISHEYE_STRENGTH: f32 = 0.55;
pub const MIN_CHART_FISHEYE_STRENGTH: f32 = 0.10;
pub const MAX_CHART_FISHEYE_STRENGTH: f32 = 1.0;
pub const DEFAULT_CHART_CHROMATIC_ABERRATION_STRENGTH: f32 = 0.50;
pub const MIN_CHART_CHROMATIC_ABERRATION_STRENGTH: f32 = 0.10;
pub const MAX_CHART_CHROMATIC_ABERRATION_STRENGTH: f32 = 1.0;
pub const DEFAULT_CHART_EDGE_BLUR_STRENGTH: f32 = 0.45;
pub const MIN_CHART_EDGE_BLUR_STRENGTH: f32 = 0.10;
pub const MAX_CHART_EDGE_BLUR_STRENGTH: f32 = 1.0;
pub const DEFAULT_CHART_CROSSHAIR_SCALE: f32 = 1.0;
pub const MIN_CHART_CROSSHAIR_SCALE: f32 = 0.5;
pub const MAX_CHART_CROSSHAIR_SCALE: f32 = 2.0;
pub const DEFAULT_CHART_HUD_ORDER_SOUND_VOLUME: f32 = 0.28;
pub const MIN_CHART_HUD_ORDER_SOUND_VOLUME: f32 = 0.0;
pub const MAX_CHART_HUD_ORDER_SOUND_VOLUME: f32 = 1.0;
pub const MIN_PANE_BORDER_THICKNESS: f32 = 1.0;
pub const MAX_PANE_BORDER_THICKNESS: f32 = 12.0;
pub const MIN_PANE_CORNER_RADIUS: f32 = 0.0;
pub const MAX_PANE_CORNER_RADIUS: f32 = 16.0;
pub const MIN_WIDGET_PADDING: f32 = 0.0;
pub const MAX_WIDGET_PADDING: f32 = 32.0;

pub fn default_ui_scale() -> f32 {
    DEFAULT_UI_SCALE
}

pub fn normalize_ui_scale(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_UI_SCALE, MAX_UI_SCALE)
    } else {
        default_ui_scale()
    }
}

pub fn default_alfred_popup_scale() -> f32 {
    DEFAULT_ALFRED_POPUP_SCALE
}

pub fn normalize_alfred_popup_scale(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_ALFRED_POPUP_SCALE, MAX_ALFRED_POPUP_SCALE)
    } else {
        default_alfred_popup_scale()
    }
}

pub fn default_chart_dotted_background_opacity() -> f32 {
    DEFAULT_CHART_DOTTED_BACKGROUND_OPACITY
}

pub fn normalize_chart_dotted_background_opacity(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(
            MIN_CHART_DOTTED_BACKGROUND_OPACITY,
            MAX_CHART_DOTTED_BACKGROUND_OPACITY,
        )
    } else {
        default_chart_dotted_background_opacity()
    }
}

pub fn default_chart_fisheye_strength() -> f32 {
    DEFAULT_CHART_FISHEYE_STRENGTH
}

pub fn normalize_chart_fisheye_strength(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_CHART_FISHEYE_STRENGTH, MAX_CHART_FISHEYE_STRENGTH)
    } else {
        default_chart_fisheye_strength()
    }
}

pub fn default_chart_chromatic_aberration_strength() -> f32 {
    DEFAULT_CHART_CHROMATIC_ABERRATION_STRENGTH
}

pub fn normalize_chart_chromatic_aberration_strength(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(
            MIN_CHART_CHROMATIC_ABERRATION_STRENGTH,
            MAX_CHART_CHROMATIC_ABERRATION_STRENGTH,
        )
    } else {
        default_chart_chromatic_aberration_strength()
    }
}

pub fn default_chart_edge_blur_strength() -> f32 {
    DEFAULT_CHART_EDGE_BLUR_STRENGTH
}

pub fn normalize_chart_edge_blur_strength(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_CHART_EDGE_BLUR_STRENGTH, MAX_CHART_EDGE_BLUR_STRENGTH)
    } else {
        default_chart_edge_blur_strength()
    }
}

pub fn default_chart_crosshair_scale() -> f32 {
    DEFAULT_CHART_CROSSHAIR_SCALE
}

pub fn normalize_chart_crosshair_scale(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_CHART_CROSSHAIR_SCALE, MAX_CHART_CROSSHAIR_SCALE)
    } else {
        default_chart_crosshair_scale()
    }
}

pub fn default_chart_hud_order_sound_volume() -> f32 {
    DEFAULT_CHART_HUD_ORDER_SOUND_VOLUME
}

pub fn normalize_chart_hud_order_sound_volume(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(
            MIN_CHART_HUD_ORDER_SOUND_VOLUME,
            MAX_CHART_HUD_ORDER_SOUND_VOLUME,
        )
    } else {
        default_chart_hud_order_sound_volume()
    }
}

pub fn default_pane_border_thickness() -> f32 {
    DEFAULT_PANE_BORDER_THICKNESS
}

pub fn default_pane_corner_radius() -> f32 {
    DEFAULT_PANE_CORNER_RADIUS
}

pub fn default_widget_padding() -> f32 {
    DEFAULT_WIDGET_PADDING
}

pub fn normalize_pane_border_thickness(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_PANE_BORDER_THICKNESS, MAX_PANE_BORDER_THICKNESS)
    } else {
        default_pane_border_thickness()
    }
}

pub fn normalize_pane_corner_radius(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_PANE_CORNER_RADIUS, MAX_PANE_CORNER_RADIUS)
    } else {
        default_pane_corner_radius()
    }
}

/// Returns the effective border radius for a widget element given the user's
/// configured pane corner radius. When corners are square (`corner_radius == 0`),
/// all child element radii resolve to `0.0`; otherwise the element's own default
/// radius is used.
pub fn effective_radius(corner_radius: f32, element_radius: f32) -> f32 {
    if corner_radius == 0.0 {
        0.0
    } else {
        element_radius
    }
}

pub fn normalize_widget_padding(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_WIDGET_PADDING, MAX_WIDGET_PADDING)
    } else {
        default_widget_padding()
    }
}

impl Default for KeroseneConfig {
    fn default() -> Self {
        Self {
            saved_layouts: Vec::new(),
            active_layout_name: None,
            credential_storage_mode: CredentialStorageMode::default(),
            encrypted_secrets: None,
            main_window_width: None,
            main_window_height: None,
            main_window_x: None,
            main_window_y: None,
            journal_window_width: None,
            journal_window_height: None,
            accounts: Vec::new(),
            active_account_index: 0,
            agent_key: String::new().into(),
            wallet_address: String::new(),
            active_symbol: default_symbol(),
            active_timeframe: default_timeframe(),
            order_kind: default_order_kind(),
            reduce_only: false,
            order_quantity_is_usd: false,
            ui_scale: default_ui_scale(),
            chart_dotted_background: false,
            chart_dotted_background_opacity: default_chart_dotted_background_opacity(),
            chart_hollow_candles: false,
            chart_hollow_candle_mode: Default::default(),
            chart_fisheye_enabled: false,
            chart_fisheye_strength: default_chart_fisheye_strength(),
            chart_chromatic_aberration_enabled: false,
            chart_chromatic_aberration_strength: default_chart_chromatic_aberration_strength(),
            chart_edge_blur_enabled: false,
            chart_edge_blur_strength: default_chart_edge_blur_strength(),
            chart_crosshair_style: Default::default(),
            chart_crosshair_guides_enabled: true,
            chart_crosshair_scale: default_chart_crosshair_scale(),
            chart_hud_order_sound: Default::default(),
            chart_hud_order_sound_file: None,
            chart_hud_order_sound_volume: default_chart_hud_order_sound_volume(),
            chart_hud_readout: Default::default(),
            alfred_popup_scale: default_alfred_popup_scale(),
            read_data_provider: Default::default(),
            chart_backfill_source: Default::default(),
            display_font: Default::default(),
            monospace_font: Default::default(),
            custom_fonts: Vec::new(),
            pane_border_thickness: default_pane_border_thickness(),
            pane_corner_radius: default_pane_corner_radius(),
            outer_widget_border_enabled: true,
            widget_padding: Default::default(),
            custom_window_chrome_enabled: true,
            book_tick_size: default_tick_size(),
            symbol_search_sort_mode: default_symbol_search_sort_mode(),
            market_universe: Default::default(),
            liquidation_distribution_symbol: String::new(),
            display_denomination: Default::default(),
            chart_screenshot_settings: Default::default(),
            layout_ratios: default_layout_ratios(),
            pane_layout: None,
            charts: Vec::new(),
            detached_chart_windows: Vec::new(),
            order_books: Vec::new(),
            live_watchlists: Vec::new(),
            positioning_infos: Vec::new(),
            session_data: Vec::new(),
            ticker_tape_enabled: false,
            favourite_symbols: Vec::new(),
            muted_tickers: Vec::new(),
            hydromancer_api_key: String::new().into(),
            hyperdash_api_key: String::new().into(),
            sound_enabled: false,
            desktop_notifications: false,
            toast_position: Default::default(),
            toast_animations_enabled: true,
            income_alerts_enabled: false,
            hide_pnl: false,
            hidden_positions_by_account: HashMap::new(),
            liquidation_alerts_enabled: false,
            liquidation_alert_threshold: default_liquidation_alert_threshold(),
            market_slippage_pct: default_market_slippage_pct(),
            optimistic_account_updates: false,
            tracked_trade_alerts_enabled: false,
            tracked_trade_aggregation_enabled: false,
            liquidation_feed_aggregation_enabled: false,
            telegram_feed_notifications_enabled: false,
            telegram_feed_fast_mode_enabled: false,
            telegram_feed_fast_api_id: None,
            telegram_feed_channels: crate::telegram_feed::default_telegram_feed_channels(),
            telegram_feed_private_channels: Vec::new(),
            x_feed_notifications_enabled: false,
            x_feed_streaming_enabled: false,
            x_feed_handles: crate::x_feed::default_x_feed_handles(),
            x_bearer_token: String::new().into(),
            spaghetti_charts: Vec::new(),
            wallet_tracker: WalletTrackerConfig {
                tracked_addresses: Vec::new(),
                muted_addresses: Vec::new(),
                wallets: Vec::new(),
                open: false,
                width: default_wallet_tracker_width(),
                height: default_wallet_tracker_height(),
                x: None,
                y: None,
            },
            address_book: Vec::new(),
            active_theme: default_theme(),
            custom_themes: default_custom_themes(),
            journal_entries: HashMap::new(),
            journal_entries_by_account: HashMap::new(),
            order_presets: OrderPresetsConfig::default(),
            advanced_order_history: Vec::new(),
            preset_is_usd: true,
            hotkeys: Vec::new(),
            chart_timeframe_hotkey_prefix: None,
        }
    }
}

use super::{CredentialStorageMode, KeroseneConfig};
use crate::config::wallets::{default_wallet_tracker_height, default_wallet_tracker_width};
use crate::config::{
    OrderPresetsConfig, WalletTrackerConfig, default_custom_themes, default_theme,
};
use crate::pane_state::{DEFAULT_PANE_BORDER_THICKNESS, DEFAULT_PANE_CORNER_RADIUS};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Config Defaults
// ---------------------------------------------------------------------------

pub fn default_symbol() -> String {
    "HYPE".to_string()
}

pub(super) fn default_true() -> bool {
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
    vec![0.06, 0.70, 0.50, 0.55, 0.65]
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

pub const MIN_PANE_BORDER_THICKNESS: f32 = 1.0;
pub const MAX_PANE_BORDER_THICKNESS: f32 = 12.0;
pub const MIN_PANE_CORNER_RADIUS: f32 = 0.0;
pub const MAX_PANE_CORNER_RADIUS: f32 = 16.0;

pub fn default_pane_border_thickness() -> f32 {
    DEFAULT_PANE_BORDER_THICKNESS
}

pub fn default_pane_corner_radius() -> f32 {
    DEFAULT_PANE_CORNER_RADIUS
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
            accounts: Vec::new(),
            active_account_index: 0,
            agent_key: String::new().into(),
            wallet_address: String::new(),
            active_symbol: default_symbol(),
            active_timeframe: default_timeframe(),
            order_kind: default_order_kind(),
            reduce_only: false,
            order_quantity_is_usd: false,
            pane_border_thickness: default_pane_border_thickness(),
            pane_corner_radius: default_pane_corner_radius(),
            book_tick_size: default_tick_size(),
            symbol_search_sort_mode: default_symbol_search_sort_mode(),
            market_universe: Default::default(),
            chart_screenshot_settings: Default::default(),
            layout_ratios: default_layout_ratios(),
            pane_layout: None,
            charts: Vec::new(),
            order_books: Vec::new(),
            live_watchlists: Vec::new(),
            positioning_infos: Vec::new(),
            favourite_symbols: Vec::new(),
            muted_tickers: Vec::new(),
            hydromancer_api_key: String::new().into(),
            hyperdash_api_key: String::new().into(),
            sound_enabled: false,
            desktop_notifications: false,
            income_alerts_enabled: false,
            hide_pnl: false,
            hidden_positions_by_account: HashMap::new(),
            liquidation_alerts_enabled: false,
            liquidation_alert_threshold: default_liquidation_alert_threshold(),
            market_slippage_pct: default_market_slippage_pct(),
            tracked_trade_alerts_enabled: false,
            tracked_trade_aggregation_enabled: false,
            liquidation_feed_aggregation_enabled: false,
            spaghetti_charts: Vec::new(),
            wallet_tracker: WalletTrackerConfig {
                tracked_addresses: Vec::new(),
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
        }
    }
}

use crate::account::AccountData;
use crate::account_state::PositionsSortColumn;
use crate::advanced_order_history::AdvancedOrderHistoryEntry;
use crate::alfred_state::AlfredState;
use crate::annotations::DrawingTool;
use crate::api::{self, ExchangeSymbol};
use crate::calendar_state::{CalendarImpactFilter, CalendarWindowFilter};
use crate::chart::ChartViewport;
use crate::chart_screenshot::ChartScreenshotState;
use crate::chart_state::{ChartId, ChartInstance, ChartSurfaceId, DetachedChartWindowState};
use crate::hype_etf_state::HypeEtfState;
use crate::hyperdash_api::LiquidationHeatmap;
use crate::market_state::{
    LiveWatchlistId, LiveWatchlistInstance, OrderBookId, OrderBookInstance,
    SymbolSearchMarketFilter, SymbolSearchSortMode,
};
use crate::notification_state::Toast;
use crate::order_execution::{PendingMoveOrderContext, PendingOrderAction};
use crate::order_pending_indicators::PendingOrderIndicator;
use crate::pane_management::AddWidgetPlacement;
use crate::pane_state::PaneKind;
use crate::pnl_card::PnlCardWindowState;
use crate::portfolio_state::{IncomeState, PortfolioState};
use crate::positioning_state::{PositioningInfoId, PositioningInfoInstance};
use crate::settings_state::SettingsTab;
use crate::signing::{ChaseOrder, OrderKind};
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiChartInstance};
use crate::timeframe::Timeframe;
use crate::twap_state::{TwapOrder, TwapOrderForm};
use crate::wallet_state::{AddressBookEntry, WalletDetailsWindowState, WalletTrackerState};
use crate::{config, journal, ws};
use iced::widget::pane_grid;
use iced::window;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

pub(crate) type SensitiveString = Zeroizing<String>;

pub(crate) fn sensitive_string(value: impl Into<String>) -> SensitiveString {
    Zeroizing::new(value.into())
}

pub(crate) struct TradingTerminal {
    pub(crate) saved_layouts: Vec<config::SavedLayout>,
    pub(crate) active_layout_name: Option<String>,
    pub(crate) layout_input: String,
    pub(crate) panes: pane_grid::State<PaneKind>,
    pub(crate) dragging_pane: Option<pane_grid::Pane>,
    pub(crate) active_theme: String,
    pub(crate) ui_scale: f32,
    pub(crate) chart_dotted_background: bool,
    pub(crate) chart_dotted_background_opacity: f32,
    pub(crate) alfred_popup_scale: f32,
    pub(crate) display_font: config::DisplayFontConfig,
    pub(crate) monospace_font: config::DisplayFontConfig,
    pub(crate) custom_fonts: Vec<config::CustomFontConfig>,
    pub(crate) pane_border_thickness: f32,
    pub(crate) pane_corner_radius: f32,
    pub(crate) hotkeys: Vec<config::HotkeyConfig>,
    pub(crate) chart_timeframe_hotkey_prefix: Option<config::HotkeyPrefixConfig>,
    pub(crate) recording_hotkey_for: Option<config::HotkeyAction>,
    pub(crate) alfred: AlfredState,
    pub(crate) focus: Option<pane_grid::Pane>,
    // Order entry form fields
    pub(crate) order_price: String,
    pub(crate) order_quantity: String,
    pub(crate) order_quantity_is_usd: bool,
    pub(crate) order_percentage: f32,
    pub(crate) order_kind: OrderKind,
    pub(crate) order_reduce_only: bool,
    // Order status feedback (message, is_error)
    pub(crate) order_status: Option<(String, bool)>,
    pub(crate) pending_order_action: Option<PendingOrderAction>,
    pub(crate) pending_move_order_contexts: HashMap<u64, PendingMoveOrderContext>,
    pub(crate) pending_order_indicators: BTreeMap<u64, PendingOrderIndicator>,
    pub(crate) active_move_order_drag: Option<u64>,
    // Order presets
    pub(crate) order_presets: crate::config::OrderPresetsConfig,
    pub(crate) presets_menu_expanded: bool,
    pub(crate) preset_is_usd: bool,
    pub(crate) preset_edit_mode: bool,
    pub(crate) preset_edit_buffer: String,
    pub(crate) preset_edit_idx: Option<(OrderKind, usize)>, // which preset is being edited
    // Multi-chart state: each chart pane has its own instance
    pub(crate) charts: HashMap<ChartId, ChartInstance>,
    pub(crate) next_chart_id: ChartId,
    // Primary chart ID - this chart follows watchlist symbol changes
    pub(crate) primary_chart_id: Option<ChartId>,
    // Spaghetti (comparison) charts
    pub(crate) spaghetti_charts: HashMap<SpaghettiChartId, SpaghettiChartInstance>,
    pub(crate) next_spaghetti_id: SpaghettiChartId,
    // Add-widget menu state
    pub(crate) add_widget_menu_open: bool,
    pub(crate) layout_menu_open: bool,
    pub(crate) layout_rename_index: Option<usize>,
    pub(crate) layout_rename_input: String,
    pub(crate) add_widget_placement: AddWidgetPlacement,
    pub(crate) account_picker_open: bool,
    pub(crate) account_picker_rename_index: Option<usize>,
    // Calendar state
    pub(crate) calendar_events: Vec<api::CalendarEvent>,
    pub(crate) calendar_error: Option<String>,
    pub(crate) calendar_last_fetch: Option<std::time::Instant>,
    pub(crate) calendar_loading: bool,
    pub(crate) calendar_retry_attempts: u8,
    pub(crate) calendar_next_retry: Option<std::time::Instant>,
    pub(crate) calendar_impact_filter: CalendarImpactFilter,
    pub(crate) calendar_window_filter: CalendarWindowFilter,
    // Active symbol (drives order entry + order book)
    pub(crate) active_symbol: String,
    pub(crate) active_symbol_display: String,
    // Symbol search (replaces old watchlist)
    pub(crate) exchange_symbols: Vec<ExchangeSymbol>,
    pub(crate) symbols_loading: bool,
    pub(crate) symbol_search_query: String,
    pub(crate) symbol_search_sort_mode: SymbolSearchSortMode,
    pub(crate) market_universe: config::MarketUniverseConfig,
    pub(crate) symbol_search_market_filter: SymbolSearchMarketFilter,
    pub(crate) symbol_search_hip3_dex_filter: Option<String>,
    pub(crate) symbol_search_result_indices: Vec<usize>,
    pub(crate) symbol_search_favourite_count: usize,
    pub(crate) symbol_search_ctxs: HashMap<String, crate::api::WatchlistContext>,
    pub(crate) symbol_search_contexts_loading: bool,
    pub(crate) symbol_search_contexts_last_fetch_ms: Option<u64>,
    pub(crate) symbol_search_status: Option<(String, bool)>,
    pub(crate) outcome_volumes_24h: HashMap<String, f64>,
    pub(crate) outcome_volumes_loading: bool,
    pub(crate) outcome_volumes_error: Option<String>,
    pub(crate) outcome_search_query: String,
    pub(crate) hype_etfs: HypeEtfState,
    pub(crate) display_denomination: config::DisplayDenominationConfig,
    // L2 order books
    pub(crate) order_books: HashMap<OrderBookId, OrderBookInstance>,
    pub(crate) next_order_book_id: OrderBookId,
    // Wallet / account connection
    pub(crate) accounts: Vec<config::AccountProfile>,
    pub(crate) active_account_index: usize,
    pub(crate) ghost_account_secret_ids: HashSet<String>,
    pub(crate) last_persisted_active_account_secret_id: Option<String>,
    pub(crate) wallet_key_input: SensitiveString,
    pub(crate) wallet_address_input: String,
    pub(crate) hydromancer_api_key: SensitiveString,
    pub(crate) hydromancer_key_input: SensitiveString,
    pub(crate) secret_store_status: Option<(String, bool)>,
    pub(crate) secret_storage_mode: config::CredentialStorageMode,
    pub(crate) secret_storage_selection: config::CredentialStorageMode,
    pub(crate) encrypted_secrets: Option<config::EncryptedSecretsConfig>,
    pub(crate) encrypted_secret_password: SensitiveString,
    pub(crate) encrypted_secret_confirm: SensitiveString,
    pub(crate) encrypted_secrets_unlocked: bool,
    pub(crate) show_unlock_credentials_popup: bool,
    pub(crate) config_cleared_this_session: bool,
    pub(crate) config_save_due_at: Option<std::time::Instant>,
    pub(crate) config_save_in_flight: bool,
    pub(crate) config_save_exit_requested: bool,
    pub(crate) liquidations: VecDeque<ws::LiquidationEvent>,
    // (long_notional, short_notional)
    pub(crate) liquidation_summary_buckets: BTreeMap<u64, (f64, f64)>,
    pub(crate) liquidations_status: String,
    pub(crate) liquidations_last_rx_ms: Option<u64>,
    pub(crate) liquidations_reconnect_nonce: u64,
    pub(crate) tracked_trades: VecDeque<ws::TrackedTradeEvent>,
    pub(crate) tracked_trades_status: String,
    pub(crate) tracked_trades_last_rx_ms: Option<u64>,
    pub(crate) tracked_trades_reconnect_nonce: u64,
    pub(crate) tracked_trade_seen_keys: HashSet<String>,
    pub(crate) tracked_trade_seen_order: VecDeque<String>,
    pub(crate) tracked_trade_aggregation_enabled: bool,
    pub(crate) tracked_trade_settings_menu_open: bool,
    pub(crate) liquidation_feed_aggregation_enabled: bool,
    pub(crate) liquidation_chart_enabled: bool,
    pub(crate) liquidation_summary_enabled: bool,
    pub(crate) liquidation_settings_menu_open: bool,
    // (long_notional, short_notional)
    pub(crate) liquidation_chart_buckets: BTreeMap<u64, (f64, f64)>,
    pub(crate) connected_address: Option<String>,
    pub(crate) account_data: Option<AccountData>,
    pub(crate) account_loading: bool,
    pub(crate) account_reconciliation_required: bool,
    pub(crate) account_error: Option<String>,
    pub(crate) account_refresh_backoff_until_ms: Option<u64>,
    // Real-time mid prices for all coins (updated via allMids WS stream)
    pub(crate) all_mids: HashMap<String, f64>,
    pub(crate) all_mids_updated_at_ms: HashMap<String, u64>,
    // Real-time tracking of price direction flashes: coin -> (timestamp_ms, direction)
    pub(crate) live_watchlist_flashes: HashMap<String, (u64, i8)>,
    // Close-position menu: which coin's menu is currently expanded (if any)
    pub(crate) close_menu_coin: Option<String>,
    pub(crate) nuke_confirmation: Option<std::time::Instant>,
    pub(crate) positions_sort_column: PositionsSortColumn,
    pub(crate) positions_sort_direction: config::SortDirection,
    pub(crate) hidden_positions_by_account: HashMap<String, HashSet<String>>,
    pub(crate) show_hidden_positions: bool,
    // Client-side chase orders. Chases run at account scope and do not depend
    // on a visible chart/order-book widget after they are started.
    pub(crate) chase_orders: BTreeMap<u64, ChaseOrder>,
    pub(crate) selected_chase_id: Option<u64>,
    pub(crate) next_chase_id: u64,
    pub(crate) twap_orders: BTreeMap<u64, TwapOrder>,
    pub(crate) selected_twap_id: Option<u64>,
    pub(crate) next_twap_id: u64,
    pub(crate) twap_form: TwapOrderForm,
    pub(crate) advanced_order_history: VecDeque<AdvancedOrderHistoryEntry>,
    pub(crate) advanced_order_history_windows: HashMap<window::Id, String>,
    pub(crate) last_advanced_exchange_request_at: Option<std::time::Instant>,
    // Hide dollar PnL values (trader focus mode)
    pub(crate) hide_pnl: bool,
    // Optional full-width favourites ticker tape.
    pub(crate) ticker_tape_enabled: bool,
    pub(crate) ticker_tape_scroll_px: f32,
    pub(crate) ticker_tape_ctxs: HashMap<String, crate::api::WatchlistContext>,
    pub(crate) ticker_tape_contexts_loading: bool,
    pub(crate) ticker_tape_contexts_last_fetch_ms: Option<u64>,
    // Favourite symbol keys (displayed at top of symbol search)
    pub(crate) favourite_symbols: Vec<String>,
    // Global risk filter for symbols the trader wants hidden everywhere.
    pub(crate) muted_tickers: HashSet<String>,
    pub(crate) muted_ticker_input: String,
    pub(crate) muted_ticker_status: Option<(String, bool)>,
    // HyperDash API key for liquidation heatmap data
    pub(crate) hyperdash_api_key: SensitiveString,
    pub(crate) hyperdash_key_input: SensitiveString,
    // Toast notification queue
    pub(crate) toasts: Vec<Toast>,
    pub(crate) next_toast_id: u64,
    // Notification settings
    pub(crate) sound_enabled: bool,
    pub(crate) desktop_notifications: bool,
    pub(crate) income_alerts_enabled: bool,
    pub(crate) last_income_alert_time: Option<u64>,
    pub(crate) liquidation_alerts_enabled: bool,
    pub(crate) liquidation_alert_threshold: f64,
    pub(crate) liquidation_alert_input: String,
    pub(crate) market_slippage_pct: f64,
    pub(crate) market_slippage_input: String,
    pub(crate) tracked_trade_alerts_enabled: bool,
    // Multi-window IDs
    pub(crate) main_window_id: Option<window::Id>,
    pub(crate) settings_window_id: Option<window::Id>,
    pub(crate) chart_screenshot_window_id: Option<window::Id>,
    pub(crate) pnl_card_windows: HashMap<window::Id, PnlCardWindowState>,
    pub(crate) detached_chart_windows: HashMap<window::Id, DetachedChartWindowState>,
    pub(crate) chart_screenshot: Option<ChartScreenshotState>,
    pub(crate) chart_screenshot_error: Option<String>,
    pub(crate) chart_screenshot_capture_in_progress: bool,
    pub(crate) chart_screenshot_next_request_id: u64,
    pub(crate) chart_screenshot_pending_request_id: Option<u64>,
    pub(crate) chart_screenshot_settings: config::ChartScreenshotSettingsConfig,
    pub(crate) chart_screenshot_menu_open: Option<ChartSurfaceId>,
    pub(crate) chart_surface_active_tools: HashMap<ChartSurfaceId, DrawingTool>,
    pub(crate) chart_surface_viewports: HashMap<ChartSurfaceId, ChartViewport>,
    pub(crate) chart_quick_order_surface: HashMap<ChartId, ChartSurfaceId>,
    pub(crate) main_window_size: Option<iced::Size>,
    pub(crate) main_window_pos: Option<iced::Point>,
    pub(crate) wallet_tracker: WalletTrackerState,
    pub(crate) wallet_detail_windows: HashMap<window::Id, WalletDetailsWindowState>,
    pub(crate) address_book: HashMap<String, AddressBookEntry>,
    pub(crate) portfolio: PortfolioState,
    pub(crate) income: IncomeState,
    pub(crate) settings_active_tab: SettingsTab,
    // Custom Themes
    pub(crate) custom_themes: Vec<config::CustomThemeConfig>,
    // Trading Journal
    pub(crate) live_watchlists: HashMap<LiveWatchlistId, LiveWatchlistInstance>,
    pub(crate) live_watchlist_settings_menu_open: Option<LiveWatchlistId>,
    pub(crate) positioning_infos: HashMap<PositioningInfoId, PositioningInfoInstance>,
    pub(crate) next_positioning_info_id: PositioningInfoId,
    pub(crate) positioning_info_pending: HashMap<String, Vec<PositioningInfoId>>,

    pub(crate) live_watchlist_ctxs: HashMap<String, crate::api::WatchlistContext>,
    pub(crate) live_watchlist_history: HashMap<String, (f64, f64, f64)>,
    pub(crate) live_watchlist_contexts_loading: bool,
    pub(crate) live_watchlist_history_loading: bool,
    pub(crate) live_watchlist_contexts_last_fetch_ms: Option<u64>,
    pub(crate) live_watchlist_history_loaded_at: HashMap<String, u64>,
    pub(crate) live_watchlist_status: Option<(String, bool)>,
    pub(crate) journal: journal::JournalState,
    // Shared loading spinner phase
    pub(crate) spinner_phase: f32,
    // Global cache for candlestick data
    pub(crate) candle_data_cache: HashMap<(String, Timeframe), Vec<api::Candle>>,
    pub(crate) candle_data_cache_order: VecDeque<(String, Timeframe)>,
    // Shared cache/dedupe for HyperDash historical heatmap requests
    pub(crate) heatmap_data_cache: HashMap<String, LiquidationHeatmap>,
    pub(crate) heatmap_data_cache_order: VecDeque<String>,
    pub(crate) heatmap_pending_charts: HashMap<String, Vec<ChartId>>,
    // Shared in-flight dedupe for HyperDash liquidation level requests
    pub(crate) liquidation_pending_charts: HashMap<String, Vec<ChartId>>,
}

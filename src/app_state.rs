use crate::account::AccountData;
use crate::account_state::{ActiveAccountSource, AddAccountWindowState, PositionsSortColumn};
use crate::advanced_order_history::AdvancedOrderHistoryEntry;
use crate::alfred_state::AlfredState;
use crate::annotations::DrawingTool;
use crate::api::{self, ExchangeSymbol};
use crate::calendar_state::{CalendarImpactFilter, CalendarWindowFilter};
use crate::chart::ChartViewport;
use crate::chart_screenshot::ChartScreenshotState;
use crate::chart_state::{ChartId, ChartInstance, ChartSurfaceId, DetachedChartWindowState};
use crate::hype_etf_state::HypeEtfState;
use crate::hype_unstaking_state::HypeUnstakingQueueState;
use crate::hyperdash_api::LiquidationHeatmap;
use crate::liquidations_distribution_state::LiquidationDistributionState;
use crate::market_state::{
    LiveWatchlistId, LiveWatchlistInstance, OrderBookId, OrderBookInstance,
    SymbolSearchMarketFilter, SymbolSearchSortMode,
};
use crate::notification_state::Toast;
use crate::order_execution::{
    HudPlacementTracker, MoveOrderKey, PendingLeverageUpdateContext, PendingMoveOrderContext,
    PendingNukeExecution, PendingOrderAction, SpotAutomationSymbolIdentity,
};
use crate::order_pending_indicators::PendingOrderIndicator;
use crate::order_update::{
    NukeConfirmation, OrderQuantityProvenance, PendingCancelStatusRequest,
    PendingMoveStatusRequest, PendingOneShotStatusRequest,
};
use crate::pane_management::AddWidgetPlacement;
use crate::pane_state::PaneKind;
use crate::pnl_card::PnlCardWindowState;
use crate::portfolio_state::{IncomeState, PortfolioState};
use crate::positioning_state::{PositioningInfoId, PositioningInfoInstance};
use crate::schwab::SchwabState;
use crate::screener_state::ScreenerState;
use crate::session_data_state::{SessionDataId, SessionDataInstance};
use crate::settings_state::{SettingsTab, ThemeSettingsPage};
use crate::signing::{ChaseOrder, OrderKind};
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiChartInstance};
use crate::telegram_feed::TelegramFeedState;
use crate::timeframe::Timeframe;
use crate::twap_state::{TwapOrder, TwapOrderForm};
use crate::wallet_cluster_state::WalletClusterState;
use crate::wallet_state::{AddressBookEntry, WalletDetailsWindowState, WalletTrackerState};
use crate::x_feed::XFeedState;
use crate::{config, journal, ws};
use iced::widget::pane_grid;
use iced::window;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::{fmt, ops::Deref, time::Instant};
use zeroize::{Zeroize, Zeroizing};

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

#[derive(Clone, Default, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct SensitiveString(Zeroizing<String>);

impl SensitiveString {
    pub(crate) fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub(crate) fn into_zeroizing(self) -> Zeroizing<String> {
        self.0
    }

    #[cfg(test)]
    pub(crate) fn clear(&mut self) {
        self.zeroize();
    }
}

impl fmt::Debug for SensitiveString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SensitiveString(<redacted>)")
    }
}

impl Deref for SensitiveString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for SensitiveString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Zeroize for SensitiveString {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl From<String> for SensitiveString {
    fn from(value: String) -> Self {
        Self(Zeroizing::new(value))
    }
}

impl From<&str> for SensitiveString {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

impl From<Zeroizing<String>> for SensitiveString {
    fn from(value: Zeroizing<String>) -> Self {
        Self(value)
    }
}

pub(crate) fn sensitive_string(value: impl Into<String>) -> SensitiveString {
    value.into().into()
}

impl TradingTerminal {
    #[cfg(test)]
    pub(crate) fn set_committed_agent_key_for_test(&mut self, value: impl Into<String>) {
        let value = value.into();
        self.wallet_key_input = sensitive_string(value.clone());
        if let Some(profile) = self.accounts.get_mut(self.active_account_index) {
            profile.agent_key = sensitive_string(value).into_zeroizing();
        }
    }

    #[cfg(test)]
    pub(crate) fn set_account_data_for_address_for_test(
        &mut self,
        account_address: impl Into<String>,
        data: AccountData,
    ) {
        self.bump_account_data_revision();
        self.bump_spot_balances_revision();
        self.account_data_address = Some(account_address.into());
        self.account_data = Some(data);
    }

    pub(crate) fn bump_account_data_revision(&mut self) {
        self.account_data_revision = self.account_data_revision.wrapping_add(1);
    }

    pub(crate) fn bump_spot_balances_revision(&mut self) {
        self.spot_balances_revision = self.spot_balances_revision.wrapping_add(1);
    }

    pub(crate) fn hydromancer_api_key_for_task(&self) -> Zeroizing<String> {
        Zeroizing::new(self.hydromancer_api_key.trim().to_string())
    }

    pub(crate) fn invalidate_portfolio_income_refreshes(&mut self) {
        self.portfolio.invalidate_refresh();
        self.income.invalidate_refresh();
    }

    pub(crate) fn clear_portfolio_income_account_state(&mut self) {
        self.invalidate_portfolio_income_refreshes();
        self.portfolio.data = None;
        self.portfolio.last_error = None;
        self.income.data = None;
        self.income.last_error = None;
        self.last_income_alert_time = None;
    }

    pub(crate) fn bump_hydromancer_key_generation(&mut self) {
        self.hydromancer_key_generation = self.hydromancer_key_generation.wrapping_add(1);
        self.invalidate_portfolio_income_refreshes();
        if self.read_data_provider == config::ReadDataProvider::Hydromancer {
            self.invalidate_wallet_read_data_requests();
        }
    }

    pub(crate) fn bump_read_data_provider_generation(&mut self) {
        self.read_data_provider_generation = self.read_data_provider_generation.wrapping_add(1);
    }

    pub(crate) fn hydromancer_key_generation_is_current(&self, generation: u64) -> bool {
        self.hydromancer_key_generation == generation
    }

    pub(crate) fn hyperdash_api_key_for_task(&self) -> Zeroizing<String> {
        Zeroizing::new(self.hyperdash_api_key.trim().to_string())
    }

    pub(crate) fn bump_hyperdash_key_generation(&mut self) {
        self.hyperdash_key_generation = self.hyperdash_key_generation.wrapping_add(1);
        self.invalidate_positioning_info_requests();
        self.invalidate_liquidation_distribution_request();
        self.invalidate_hyperdash_chart_requests_for_key_change();
    }

    pub(crate) fn hyperdash_key_generation_is_current(&self, generation: u64) -> bool {
        self.hyperdash_key_generation == generation
    }

    pub(crate) fn openrouter_api_key_for_task(&self) -> Zeroizing<String> {
        Zeroizing::new(self.openrouter_api_key.trim().to_string())
    }

    /// Whether AI features backed by OpenRouter can run.
    #[allow(dead_code)] // availability check for upcoming AI components
    pub(crate) fn openrouter_configured(&self) -> bool {
        !self.openrouter_api_key.trim().is_empty()
    }

    /// Model slug AI components should request: the configured default model,
    /// or OpenRouter's auto router when none is set.
    #[allow(dead_code)] // model selection for upcoming AI components
    pub(crate) fn openrouter_model_for_task(&self) -> String {
        let model = self.openrouter_model.trim();
        if model.is_empty() {
            crate::openrouter_api::DEFAULT_OPENROUTER_MODEL.to_string()
        } else {
            model.to_string()
        }
    }

    pub(crate) fn bump_openrouter_key_generation(&mut self) {
        self.openrouter_key_generation = self.openrouter_key_generation.wrapping_add(1);
        self.openrouter_key_status = None;
    }

    pub(crate) fn openrouter_key_generation_is_current(&self, generation: u64) -> bool {
        self.openrouter_key_generation == generation
    }

    pub(crate) fn invalidate_positioning_info_requests(&mut self) {
        self.positioning_info_pending.clear();
        for instance in self.positioning_infos.values_mut() {
            if instance.pending_key.is_some() {
                instance.loading = false;
                instance.pending_key = None;
            }
            if instance.change_pending_key.is_some() {
                instance.change_loading = false;
                instance.change_pending_key = None;
            }
        }
    }

    fn invalidate_liquidation_distribution_request(&mut self) {
        self.liquidation_distribution.loading = false;
        self.liquidation_distribution.pending_request = None;
    }
}

pub(crate) struct TradingTerminal {
    pub(crate) saved_layouts: Vec<config::SavedLayout>,
    pub(crate) active_layout_name: Option<String>,
    pub(crate) app_onboarding_dismissed: bool,
    pub(crate) layout_input: String,
    pub(crate) preserved_loaded_pane_layout: Option<config::PaneLayoutConfig>,
    pub(crate) panes: pane_grid::State<PaneKind>,
    pub(crate) dragging_pane: Option<pane_grid::Pane>,
    pub(crate) active_theme: String,
    pub(crate) ui_scale: f32,
    pub(crate) chart_dotted_background: bool,
    pub(crate) chart_dotted_background_opacity: f32,
    pub(crate) chart_gradient_background: bool,
    pub(crate) chart_gradient_contrast: f32,
    pub(crate) chart_hollow_candle_mode: config::ChartHollowCandleMode,
    pub(crate) chart_series_style: config::ChartSeriesStyle,
    pub(crate) chart_fisheye_enabled: bool,
    pub(crate) chart_fisheye_strength: f32,
    pub(crate) chart_chromatic_aberration_enabled: bool,
    pub(crate) chart_chromatic_aberration_strength: f32,
    pub(crate) chart_edge_blur_enabled: bool,
    pub(crate) chart_edge_blur_strength: f32,
    pub(crate) chart_crosshair_style: config::ChartCrosshairStyle,
    pub(crate) chart_crosshair_guides_enabled: bool,
    pub(crate) chart_crosshair_scale: f32,
    pub(crate) chart_hud_order_sound: config::ChartHudOrderSound,
    pub(crate) chart_hud_order_sound_file: Option<String>,
    pub(crate) chart_hud_order_sound_volume: f32,
    pub(crate) chart_hud_ui_sounds: bool,
    pub(crate) chart_hud_readout: config::ChartHudReadoutConfig,
    pub(crate) alfred_popup_scale: f32,
    pub(crate) read_data_provider: config::ReadDataProvider,
    pub(crate) read_data_provider_generation: u64,
    pub(crate) chart_backfill_source: config::ChartBackfillSource,
    pub(crate) display_font: config::DisplayFontConfig,
    pub(crate) monospace_font: config::DisplayFontConfig,
    pub(crate) custom_fonts: Vec<config::CustomFontConfig>,
    pub(crate) pane_border_thickness: f32,
    pub(crate) pane_corner_radius: f32,
    pub(crate) outer_widget_border_enabled: bool,
    pub(crate) widget_padding_default: f32,
    pub(crate) widget_padding_overrides: BTreeMap<config::WidgetPaddingTargetConfig, f32>,
    pub(crate) custom_window_chrome_enabled: bool,
    pub(crate) custom_window_chrome_active: bool,
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
    pub(crate) order_quantity_provenance: Option<OrderQuantityProvenance>,
    pub(crate) order_kind: OrderKind,
    pub(crate) order_reduce_only: bool,
    pub(crate) order_leverage_input: String,
    pub(crate) order_leverage_is_cross: bool,
    pub(crate) order_leverage_dropdown_open: bool,
    pub(crate) pending_leverage_update: Option<PendingLeverageUpdateContext>,
    // Order status feedback (message, is_error)
    pub(crate) order_status: Option<(String, bool)>,
    pub(crate) next_order_lifecycle_request_id: u64,
    pub(crate) pending_one_shot_status_requests: BTreeMap<u64, PendingOneShotStatusRequest>,
    pub(crate) pending_cancel_status_request: Option<PendingCancelStatusRequest>,
    pub(crate) pending_move_status_request: Option<PendingMoveStatusRequest>,
    pub(crate) pending_order_action: Option<PendingOrderAction>,
    pub(crate) pending_move_order_contexts: HashMap<MoveOrderKey, PendingMoveOrderContext>,
    pub(crate) pending_order_indicators: BTreeMap<u64, PendingOrderIndicator>,
    pub(crate) hud_placements: HudPlacementTracker,
    pub(crate) active_move_order_drag: Option<MoveOrderKey>,
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
    pub(crate) calendar_request_id: u64,
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
    pub(crate) exchange_symbols_refresh_inflight: bool,
    /// Runtime-only generation for cached startup, immediate live verification,
    /// and periodic exchange-symbol metadata requests.
    pub(crate) exchange_symbols_request_id: u64,
    /// A spot metadata refresh failed validation or transport. Previously
    /// loaded spot markets remain visible, but new orders are fail-closed
    /// until a complete `spotMeta` response is verified.
    pub(crate) spot_metadata_degraded: bool,
    /// Global guard for the full-universe spot chart-context endpoint. This is
    /// separate from per-chart missing-symbol backoff so an endpoint outage or
    /// HTTP 429 cannot be bypassed by opening another chart.
    pub(crate) spot_asset_context_rest_in_flight: bool,
    pub(crate) spot_asset_context_rest_failures: u8,
    pub(crate) spot_asset_context_rest_next_attempt_at_ms: Option<u64>,
    /// Persisted display labels for outcome trade coins ("#NNN" -> label) so
    /// expired or not-yet-loaded HIP-4 markets keep their human-readable names.
    pub(crate) outcome_display_labels: HashMap<String, String>,
    pub(crate) symbol_search_query: String,
    pub(crate) symbol_search_sort_mode: SymbolSearchSortMode,
    pub(crate) market_universe: config::MarketUniverseConfig,
    pub(crate) symbol_search_market_filter: SymbolSearchMarketFilter,
    pub(crate) symbol_search_hip3_dex_filter: Option<String>,
    pub(crate) symbol_search_result_indices: Vec<usize>,
    pub(crate) symbol_search_favourite_count: usize,
    pub(crate) symbol_search_ctxs: HashMap<String, crate::api::WatchlistContext>,
    pub(crate) symbol_search_contexts_loading: bool,
    pub(crate) symbol_search_contexts_request_id: u64,
    pub(crate) symbol_search_contexts_request_symbols: Vec<String>,
    pub(crate) symbol_search_contexts_refresh_pending: bool,
    pub(crate) symbol_search_contexts_last_fetch_ms: Option<u64>,
    pub(crate) symbol_search_status: Option<(String, bool)>,
    pub(crate) outcome_volumes_24h: HashMap<String, api::OutcomeVolume24h>,
    pub(crate) outcome_volumes_loading: bool,
    pub(crate) outcome_volumes_request_id: u64,
    pub(crate) outcome_volumes_request_symbols: Vec<String>,
    pub(crate) outcome_volumes_error: Option<String>,
    pub(crate) outcome_search_query: String,
    pub(crate) outcome_collapsed_market_groups: HashSet<String>,
    pub(crate) hype_etfs: HypeEtfState,
    pub(crate) hype_unstaking_queue: HypeUnstakingQueueState,
    pub(crate) display_denomination: config::DisplayDenominationConfig,
    // L2 order books
    pub(crate) order_books: HashMap<OrderBookId, OrderBookInstance>,
    pub(crate) next_order_book_id: OrderBookId,
    /// Terminal-lifetime sequence for REST book snapshots. It deliberately
    /// outlives individual pane instances and runtime layout reconstruction.
    pub(crate) next_order_book_request_id: u64,
    // Wallet / account connection
    pub(crate) accounts: Vec<config::AccountProfile>,
    pub(crate) pending_keychain_profile_deletions: Vec<String>,
    pub(crate) pending_keychain_cleanup_all: bool,
    pub(crate) active_account_index: usize,
    pub(crate) active_account_source: ActiveAccountSource,
    pub(crate) ghost_account_secret_ids: HashSet<String>,
    pub(crate) last_persisted_active_account_secret_id: Option<String>,
    pub(crate) wallet_key_input: SensitiveString,
    pub(crate) wallet_address_input: String,
    pub(crate) hydromancer_api_key: SensitiveString,
    pub(crate) hydromancer_key_generation: u64,
    pub(crate) hydromancer_key_input: SensitiveString,
    pub(crate) secret_store_status: Option<(String, bool)>,
    pub(crate) secret_storage_mode: config::CredentialStorageMode,
    pub(crate) secret_storage_selection: config::CredentialStorageMode,
    pub(crate) encrypted_secrets: Option<config::EncryptedSecretsConfig>,
    pub(crate) encrypted_secret_password: SensitiveString,
    pub(crate) encrypted_secret_confirm: SensitiveString,
    pub(crate) encrypted_secrets_unlocked: bool,
    pub(crate) show_unlock_credentials_popup: bool,
    pub(crate) secret_migration_save_blocked: bool,
    pub(crate) config_clear_requested: bool,
    pub(crate) config_cleared_this_session: bool,
    pub(crate) config_save_due_at: Option<std::time::Instant>,
    pub(crate) config_save_in_flight: bool,
    /// The main window has closed and the final persistence/exit sequence owns
    /// the daemon. Also fences fresh exchange mutations, new destructive
    /// persistence requests, and exposure-progressing automation until exit
    /// completes or a save failure explicitly clears exit ownership.
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
    // Whether the liquidation feed auto-scrolls to the latest rows
    pub(crate) liquidation_feed_following: bool,
    // (long_notional, short_notional)
    pub(crate) liquidation_chart_buckets: BTreeMap<u64, (f64, f64)>,
    pub(crate) connected_address: Option<String>,
    /// Runtime-only recipe incarnations used to reject already-queued frames
    /// after iced replaces a user-data subscription.
    pub(crate) account_user_data_stream_generation: u64,
    pub(crate) wallet_detail_user_data_stream_generations: HashMap<String, u64>,
    pub(crate) next_wallet_detail_user_data_stream_generation: u64,
    pub(crate) wallet_cluster_user_data_stream_generation: u64,
    pub(crate) account_data: Option<AccountData>,
    pub(crate) account_data_address: Option<String>,
    pub(crate) account_data_revision: u64,
    /// Advances only when the connected account's spot balance snapshot is
    /// replaced. Spot percentage sizing must not be coupled to unrelated
    /// perp, order, fill, or funding updates in `account_data_revision`.
    pub(crate) spot_balances_revision: u64,
    pub(crate) account_loading: bool,
    /// Transient: a wallet connect has been dispatched (account switch / boot)
    /// but not yet processed. Bridges the one-frame gap where `connected_address`
    /// is `None` so the summary shows the connecting skeleton instead of flashing
    /// the disconnected add-account form.
    pub(crate) account_connect_pending: bool,
    pub(crate) account_data_request_generation: u64,
    pub(crate) account_twap_reconciliation_generations: HashMap<String, u64>,
    // A refresh was requested while one was already in flight; run one
    // follow-up when the in-flight fetch lands so the request isn't dropped.
    pub(crate) account_refresh_followup_pending: bool,
    pub(crate) account_reconciliation_required: bool,
    pub(crate) account_error: Option<String>,
    pub(crate) account_refresh_backoff_until_ms: Option<u64>,
    pub(crate) account_refresh_retry_due_ms: Option<u64>,
    // Real-time mid prices for all coins (updated via allMids WS stream)
    pub(crate) all_mids: HashMap<String, f64>,
    pub(crate) all_mids_updated_at_ms: HashMap<String, u64>,
    // Real-time tracking of price direction flashes: coin -> (timestamp_ms, direction)
    pub(crate) live_watchlist_flashes: HashMap<String, (u64, i8)>,
    // Close-position menu: which coin's menu is currently expanded (if any)
    pub(crate) close_menu_coin: Option<String>,
    pub(crate) nuke_confirmation: Option<NukeConfirmation>,
    pub(crate) pending_nuke_execution: Option<PendingNukeExecution>,
    pub(crate) next_nuke_execution_id: u64,
    pub(crate) positions_sort_column: PositionsSortColumn,
    pub(crate) positions_sort_direction: config::SortDirection,
    pub(crate) hidden_positions_by_account: HashMap<String, HashSet<String>>,
    pub(crate) show_hidden_positions: bool,
    // Client-side chase orders. Chases run at account scope and do not depend
    // on a visible chart/order-book widget after they are started.
    pub(crate) chase_orders: BTreeMap<u64, ChaseOrder>,
    pub(crate) chase_spot_symbol_identities: HashMap<u64, SpotAutomationSymbolIdentity>,
    pub(crate) selected_chase_id: Option<u64>,
    pub(crate) next_chase_id: u64,
    pub(crate) twap_orders: BTreeMap<u64, TwapOrder>,
    pub(crate) twap_spot_symbol_identities: HashMap<u64, SpotAutomationSymbolIdentity>,
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
    pub(crate) ticker_tape_contexts_request_id: u64,
    pub(crate) ticker_tape_contexts_request_symbols: Vec<String>,
    pub(crate) ticker_tape_contexts_refresh_pending: bool,
    pub(crate) ticker_tape_contexts_last_fetch_ms: Option<u64>,
    // Favourite symbol keys (displayed at top of symbol search)
    pub(crate) favourite_symbols: Vec<String>,
    // Global risk filter for symbols the trader wants hidden everywhere.
    pub(crate) muted_tickers: HashSet<String>,
    pub(crate) muted_ticker_input: String,
    pub(crate) muted_ticker_status: Option<(String, bool)>,
    // HyperDash API key for liquidation heatmap data
    pub(crate) hyperdash_api_key: SensitiveString,
    pub(crate) hyperdash_key_generation: u64,
    pub(crate) hyperdash_key_input: SensitiveString,
    // OpenRouter API key and default model for AI summaries
    pub(crate) openrouter_api_key: SensitiveString,
    pub(crate) openrouter_key_generation: u64,
    pub(crate) openrouter_key_input: SensitiveString,
    pub(crate) openrouter_key_status: Option<(String, bool)>,
    pub(crate) openrouter_model: String,
    // Toast notification queue
    pub(crate) toasts: Vec<Toast>,
    pub(crate) next_toast_id: u64,
    pub(crate) toast_position: config::ToastPosition,
    pub(crate) toast_animations_enabled: bool,
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
    pub(crate) optimistic_account_updates: bool,
    pub(crate) hydromancer_realtime_position_pnl_enabled: bool,
    pub(crate) tracked_trade_alerts_enabled: bool,
    // Multi-window IDs
    pub(crate) main_window_id: Option<window::Id>,
    pub(crate) settings_window_id: Option<window::Id>,
    pub(crate) add_account_window: Option<AddAccountWindowState>,
    pub(crate) screener: ScreenerState,
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
    pub(crate) wallet_clusters: WalletClusterState,
    pub(crate) wallet_detail_windows: HashMap<window::Id, WalletDetailsWindowState>,
    pub(crate) address_book: HashMap<String, AddressBookEntry>,
    pub(crate) hovered_wallet_address_actions: Option<String>,
    pub(crate) portfolio: PortfolioState,
    pub(crate) income: IncomeState,
    pub(crate) settings_active_tab: SettingsTab,
    pub(crate) settings_theme_page: ThemeSettingsPage,
    // Custom Themes
    pub(crate) custom_themes: Vec<config::CustomThemeConfig>,
    // Trading Journal
    pub(crate) live_watchlists: HashMap<LiveWatchlistId, LiveWatchlistInstance>,
    pub(crate) live_watchlist_settings_menu_open: Option<LiveWatchlistId>,
    pub(crate) positioning_infos: HashMap<PositioningInfoId, PositioningInfoInstance>,
    pub(crate) next_positioning_info_id: PositioningInfoId,
    pub(crate) positioning_info_pending: HashMap<String, Vec<PositioningInfoId>>,
    pub(crate) session_data: HashMap<SessionDataId, SessionDataInstance>,
    pub(crate) next_session_data_id: SessionDataId,

    pub(crate) live_watchlist_ctxs: HashMap<String, crate::api::WatchlistContext>,
    pub(crate) live_watchlist_history: HashMap<String, (f64, f64, f64)>,
    pub(crate) live_watchlist_contexts_loading: bool,
    pub(crate) live_watchlist_history_loading: bool,
    pub(crate) live_watchlist_contexts_request_id: u64,
    pub(crate) live_watchlist_contexts_request_symbols: Vec<String>,
    pub(crate) live_watchlist_contexts_refresh_pending: bool,
    pub(crate) live_watchlist_history_request_id: u64,
    pub(crate) live_watchlist_history_request_symbols: Vec<String>,
    pub(crate) live_watchlist_history_refresh_pending: bool,
    pub(crate) live_watchlist_contexts_last_fetch_ms: Option<u64>,
    pub(crate) live_watchlist_history_loaded_at: HashMap<String, u64>,
    pub(crate) live_watchlist_status: Option<(String, bool)>,
    pub(crate) journal: journal::JournalState,
    // Shared loading spinner phase
    pub(crate) spinner_phase: f32,
    // First-run onboarding animation phase; advances continuously (does not wrap
    // at TAU like spinner_phase) so the looping welcome visuals stay seamless.
    pub(crate) onboarding_phase: f32,
    // Last status bar tick timestamp, used by render code that displays wall-clock state.
    pub(crate) status_bar_now_ms: u64,
    // Last status bar tick instant, used by render code that displays monotonic timers.
    pub(crate) status_bar_now: Instant,
    // Global cache for candlestick data
    pub(crate) candle_data_cache: HashMap<(String, Timeframe), Vec<api::Candle>>,
    pub(crate) candle_data_cache_order: VecDeque<(String, Timeframe)>,
    // Shared cache/dedupe for HyperDash historical heatmap requests
    pub(crate) heatmap_data_cache: HashMap<String, LiquidationHeatmap>,
    pub(crate) heatmap_data_cache_order: VecDeque<String>,
    pub(crate) heatmap_pending_charts: HashMap<String, Vec<ChartId>>,
    // Shared cache/dedupe for SEC earnings-event requests
    pub(crate) sec_earnings_cache: HashMap<String, Vec<api::SecEarningsEvent>>,
    pub(crate) sec_earnings_cache_order: VecDeque<String>,
    pub(crate) sec_earnings_request_id: u64,
    pub(crate) sec_earnings_pending_request_ids: HashMap<String, u64>,
    pub(crate) sec_earnings_pending_charts: HashMap<String, Vec<ChartId>>,
    // Shared cache/dedupe for SEC filing-summary requests
    pub(crate) sec_filing_summary_cache: HashMap<String, api::SecFilingSummary>,
    pub(crate) sec_filing_summary_cache_order: VecDeque<String>,
    pub(crate) sec_filing_summary_request_id: u64,
    pub(crate) sec_filing_summary_pending_request_ids: HashMap<String, u64>,
    pub(crate) sec_filing_summary_pending_charts: HashMap<String, Vec<ChartId>>,
    // Shared in-flight dedupe for HyperDash liquidation level requests
    pub(crate) liquidation_pending_charts: HashMap<String, Vec<ChartId>>,
    pub(crate) liquidation_distribution: LiquidationDistributionState,
    pub(crate) telegram_feed: TelegramFeedState,
    pub(crate) x_feed: XFeedState,
    pub(crate) schwab: SchwabState,
}

#[cfg(test)]
mod tests {
    use super::{SensitiveString, sensitive_string};
    use zeroize::Zeroize;

    #[test]
    fn sensitive_string_debug_redacts_value_and_keeps_explicit_access() {
        let mut secret = sensitive_string("super-secret");

        let rendered = format!("{secret:?}");

        assert_eq!(secret.as_str(), "super-secret");
        assert_eq!(secret.trim(), "super-secret");
        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("super-secret"));

        secret.zeroize();
        assert!(secret.is_empty());
    }

    #[test]
    fn sensitive_string_can_move_into_task_owned_zeroizing_string() {
        let secret = SensitiveString::from("task-secret");

        let zeroizing = secret.into_zeroizing();

        assert_eq!(zeroizing.as_str(), "task-secret");
    }
}

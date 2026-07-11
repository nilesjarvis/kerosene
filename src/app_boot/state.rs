use crate::account_state::PositionsSortColumn;
use crate::advanced_order_history::prune_advanced_order_history;
use crate::app_state::TradingTerminal;
use crate::calendar_state::{CalendarImpactFilter, CalendarWindowFilter};
use crate::chart_state::{ChartId, ChartInstance};
use crate::config::{self, KeroseneConfig};
use crate::journal;
use crate::market_state::{SymbolSearchMarketFilter, SymbolSearchSortMode};
use crate::order_execution::HudPlacementTracker;
use crate::pane_management::AddWidgetPlacement;
use crate::pane_state::PaneKind;
use crate::portfolio_state::{IncomeState, PortfolioState};
use crate::settings_state::{SettingsTab, ThemeSettingsPage};
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiChartInstance};
use crate::wallet_cluster_state::WalletClusterState;
use crate::wallet_state::{AddressBookEntry, WalletTrackerState};

use iced::widget::pane_grid;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::time::Instant;

use super::accounts::BootAccountProfile;
use super::symbols::BootSymbolSelection;

pub(super) struct BootStateParts<'a> {
    pub(super) cfg: &'a KeroseneConfig,
    pub(super) boot_symbols: BootSymbolSelection,
    pub(super) boot_account: BootAccountProfile,
    pub(super) initial_secret_store_status: Option<(String, bool)>,
    pub(super) pane_config: pane_grid::Configuration<PaneKind>,
    pub(super) charts: HashMap<ChartId, ChartInstance>,
    pub(super) next_chart_id: ChartId,
    pub(super) spaghetti_charts: HashMap<SpaghettiChartId, SpaghettiChartInstance>,
    pub(super) next_spaghetti_id: SpaghettiChartId,
    pub(super) wallet_tracker: WalletTrackerState,
    pub(super) address_book: HashMap<String, AddressBookEntry>,
}

impl TradingTerminal {
    pub(super) fn boot_state(parts: BootStateParts<'_>) -> Self {
        let cfg = parts.cfg;
        let boot_account = parts.boot_account;
        let boot_symbols = parts.boot_symbols;
        let muted_tickers = boot_symbols.muted_tickers;
        let favourite_symbols = cfg
            .favourite_symbols
            .iter()
            .filter(|symbol| !Self::key_matches_muted_tickers(&[], &muted_tickers, symbol))
            .cloned()
            .collect();
        let live_watchlists = Self::boot_live_watchlists(cfg, &muted_tickers);
        let widget_padding = cfg.widget_padding.clone().normalized();
        let widget_padding_overrides = widget_padding
            .overrides
            .iter()
            .map(|item| (item.target.clone(), item.padding_px))
            .collect();
        let mut advanced_order_history = VecDeque::from(cfg.advanced_order_history.clone());
        prune_advanced_order_history(&mut advanced_order_history);

        let mut state = Self {
            saved_layouts: cfg.saved_layouts.clone(),
            active_layout_name: cfg.active_layout_name.clone(),
            app_onboarding_dismissed: cfg.app_onboarding_dismissed,
            layout_input: String::new(),
            preserved_loaded_pane_layout: cfg.pane_layout.clone(),
            live_watchlist_ctxs: HashMap::new(),
            live_watchlist_history: HashMap::new(),
            live_watchlist_contexts_loading: false,
            live_watchlist_history_loading: false,
            live_watchlist_contexts_request_id: 0,
            live_watchlist_contexts_request_symbols: Vec::new(),
            live_watchlist_contexts_refresh_pending: false,
            live_watchlist_history_request_id: 0,
            live_watchlist_history_request_symbols: Vec::new(),
            live_watchlist_history_refresh_pending: false,
            live_watchlist_contexts_last_fetch_ms: None,
            live_watchlist_history_loaded_at: HashMap::new(),
            live_watchlist_status: None,
            live_watchlist_settings_menu_open: None,
            panes: pane_grid::State::with_configuration(parts.pane_config),
            dragging_pane: None,
            active_theme: cfg.active_theme.clone(),
            ui_scale: cfg.ui_scale,
            chart_dotted_background: cfg.chart_dotted_background,
            chart_dotted_background_opacity: cfg.chart_dotted_background_opacity,
            chart_gradient_background: cfg.chart_gradient_background,
            chart_gradient_contrast: config::normalize_chart_gradient_contrast(
                cfg.chart_gradient_contrast,
            ),
            chart_hollow_candle_mode: cfg.chart_hollow_candle_mode,
            chart_series_style: cfg.chart_series_style,
            chart_fisheye_enabled: cfg.chart_fisheye_enabled,
            chart_fisheye_strength: config::normalize_chart_fisheye_strength(
                cfg.chart_fisheye_strength,
            ),
            chart_chromatic_aberration_enabled: cfg.chart_chromatic_aberration_enabled,
            chart_chromatic_aberration_strength:
                config::normalize_chart_chromatic_aberration_strength(
                    cfg.chart_chromatic_aberration_strength,
                ),
            chart_edge_blur_enabled: cfg.chart_edge_blur_enabled,
            chart_edge_blur_strength: config::normalize_chart_edge_blur_strength(
                cfg.chart_edge_blur_strength,
            ),
            chart_crosshair_style: cfg.chart_crosshair_style.normalized(),
            chart_crosshair_guides_enabled: cfg.chart_crosshair_guides_enabled,
            chart_crosshair_scale: config::normalize_chart_crosshair_scale(
                cfg.chart_crosshair_scale,
            ),
            chart_hud_order_sound: cfg.chart_hud_order_sound,
            chart_hud_order_sound_file: cfg.chart_hud_order_sound_file.clone(),
            chart_hud_order_sound_import_request: None,
            chart_hud_order_sound_volume: config::normalize_chart_hud_order_sound_volume(
                cfg.chart_hud_order_sound_volume,
            ),
            chart_hud_ui_sounds: cfg.chart_hud_ui_sounds,
            chart_hud_readout: cfg.chart_hud_readout,
            alfred_popup_scale: cfg.alfred_popup_scale,
            read_data_provider: cfg.read_data_provider,
            read_data_provider_generation: 0,
            chart_backfill_source: cfg.read_data_provider.chart_backfill_source(),
            display_font: cfg.display_font.clone(),
            monospace_font: cfg.monospace_font.clone(),
            custom_fonts: cfg.custom_fonts.clone(),
            preference_asset_import_next_request_id: 0,
            display_font_import_request: None,
            monospace_font_import_request: None,
            pane_border_thickness: cfg.pane_border_thickness,
            pane_corner_radius: cfg.pane_corner_radius,
            outer_widget_border_enabled: cfg.outer_widget_border_enabled,
            widget_padding_default: widget_padding.default_px,
            widget_padding_overrides,
            custom_window_chrome_enabled: cfg.custom_window_chrome_enabled,
            custom_window_chrome_active: cfg.custom_window_chrome_enabled,
            focus: None,
            order_price: String::new(),
            order_quantity: String::new(),
            order_quantity_is_usd: cfg.order_quantity_is_usd,
            order_percentage: 0.0,
            order_quantity_provenance: None,
            order_kind: boot_symbols.order_kind,
            order_reduce_only: cfg.reduce_only,
            order_leverage_input: "1".to_string(),
            order_leverage_is_cross: true,
            order_leverage_dropdown_open: false,
            pending_leverage_update: None,
            order_status: None,
            next_order_lifecycle_request_id: 0,
            pending_one_shot_status_requests: BTreeMap::new(),
            pending_cancel_status_request: None,
            pending_move_status_request: None,
            pending_order_action: None,
            pending_move_order_contexts: HashMap::new(),
            pending_order_indicators: BTreeMap::new(),
            hud_placements: HudPlacementTracker::default(),
            active_move_order_drag: None,
            order_presets: cfg.order_presets.clone(),
            preset_is_usd: cfg.preset_is_usd,
            presets_menu_expanded: false,
            preset_edit_mode: false,
            preset_edit_buffer: String::new(),
            preset_edit_idx: None,
            charts: parts.charts,
            next_chart_id: parts.next_chart_id,
            chart_instance_generation: 0,
            primary_chart_id: None,
            spaghetti_charts: parts.spaghetti_charts,
            next_spaghetti_id: parts.next_spaghetti_id,
            add_widget_menu_open: false,
            layout_menu_open: false,
            layout_rename_index: None,
            layout_rename_input: String::new(),
            add_widget_placement: AddWidgetPlacement::Below,
            account_picker_open: false,
            account_picker_rename_index: None,
            calendar_events: Vec::new(),
            calendar_error: None,
            calendar_last_fetch: None,
            calendar_loading: false,
            calendar_request_id: 0,
            calendar_retry_attempts: 0,
            calendar_next_retry: None,
            calendar_impact_filter: CalendarImpactFilter::MediumHigh,
            calendar_window_filter: CalendarWindowFilter::Upcoming,
            active_symbol: boot_symbols.active_symbol,
            active_symbol_display: boot_symbols.active_symbol_display,
            exchange_symbols: Vec::new(),
            symbols_loading: true,
            exchange_symbols_refresh_inflight: false,
            exchange_symbols_request_id: 0,
            spot_metadata_degraded: false,
            next_chart_asset_context_rest_request_id: 0,
            spot_asset_context_rest_request: None,
            spot_asset_context_rest_failures: 0,
            spot_asset_context_rest_next_attempt_at_ms: None,
            outcome_display_labels: cfg.outcome_display_labels.clone(),
            symbol_search_query: String::new(),
            symbol_search_sort_mode: SymbolSearchSortMode::from_config_str(
                &cfg.symbol_search_sort_mode,
            ),
            market_universe: cfg.market_universe.clone().normalized(),
            symbol_search_market_filter: SymbolSearchMarketFilter::default(),
            symbol_search_hip3_dex_filter: None,
            symbol_search_result_indices: Vec::new(),
            symbol_search_favourite_count: 0,
            symbol_search_ctxs: HashMap::new(),
            symbol_search_contexts_loading: false,
            symbol_search_contexts_request_id: 0,
            symbol_search_contexts_request_symbols: Vec::new(),
            symbol_search_contexts_refresh_pending: false,
            symbol_search_contexts_last_fetch_ms: None,
            symbol_search_status: None,
            outcome_volumes_24h: HashMap::new(),
            outcome_volumes_loading: false,
            outcome_volumes_request_id: 0,
            outcome_volumes_request_symbols: Vec::new(),
            outcome_volumes_error: None,
            outcome_search_query: String::new(),
            outcome_collapsed_market_groups: HashSet::new(),
            hype_etfs: crate::hype_etf_state::HypeEtfState::default(),
            hype_unstaking_queue: crate::hype_unstaking_state::HypeUnstakingQueueState::default(),
            display_denomination: cfg.display_denomination.clone().normalized(),
            order_books: HashMap::new(),
            next_order_book_id: 0,
            next_order_book_request_id: 0,
            accounts: cfg.accounts.clone(),
            pending_keychain_profile_deletions: cfg.pending_keychain_profile_deletions.clone(),
            pending_keychain_cleanup_all: cfg.pending_keychain_cleanup_all,
            active_account_index: boot_account.active_account_index,
            active_account_source: crate::account_state::ActiveAccountSource::Hyperliquid,
            ghost_account_secret_ids: HashSet::new(),
            last_persisted_active_account_secret_id: boot_account.last_persisted_secret_id,
            wallet_key_input: boot_account.agent_key.into(),
            wallet_address_input: boot_account.wallet_address,
            hydromancer_api_key: boot_account.hydromancer_key.clone().into(),
            hydromancer_key_generation: 0,
            hydromancer_key_input: boot_account.hydromancer_key.into(),
            secret_store_status: parts.initial_secret_store_status,
            secret_storage_mode: cfg.credential_storage_mode,
            secret_storage_selection: cfg.credential_storage_mode,
            encrypted_secrets: cfg.encrypted_secrets.clone(),
            encrypted_secret_password: crate::app_state::sensitive_string(String::new()),
            encrypted_secret_confirm: crate::app_state::sensitive_string(String::new()),
            encrypted_secrets_unlocked: false,
            show_unlock_credentials_popup: boot_account.show_unlock_credentials_popup,
            secret_migration_save_blocked: cfg.secret_migration_save_blocked,
            config_clear_requested: false,
            config_cleared_this_session: false,
            config_save_due_at: None,
            config_save_in_flight: false,
            config_save_exit_requested: false,
            liquidations: VecDeque::new(),
            liquidation_summary_buckets: BTreeMap::new(),
            liquidations_status: "Disconnected".to_string(),
            liquidations_last_rx_ms: None,
            liquidations_reconnect_nonce: 0,
            tracked_trades: VecDeque::new(),
            tracked_trades_status: "Disconnected".to_string(),
            tracked_trades_last_rx_ms: None,
            tracked_trades_reconnect_nonce: 0,
            tracked_trade_seen_keys: HashSet::new(),
            tracked_trade_seen_order: VecDeque::new(),
            tracked_trade_aggregation_enabled: cfg.tracked_trade_aggregation_enabled,
            tracked_trade_settings_menu_open: false,
            liquidation_feed_aggregation_enabled: cfg.liquidation_feed_aggregation_enabled,
            liquidation_chart_enabled: false,
            liquidation_summary_enabled: true,
            liquidation_settings_menu_open: false,
            liquidation_feed_following: true,
            liquidation_chart_buckets: BTreeMap::new(),
            connected_address: None,
            account_user_data_stream_generation: 0,
            wallet_detail_user_data_stream_generations: HashMap::new(),
            next_wallet_detail_user_data_stream_generation: 1,
            wallet_cluster_user_data_stream_generation: 0,
            account_data: None,
            account_data_address: None,
            account_data_revision: 0,
            spot_balances_revision: 0,
            account_loading: boot_account.has_wallet,
            account_connect_pending: false,
            account_data_request_generation: 0,
            account_twap_reconciliation_generations: HashMap::new(),
            account_refresh_followup_pending: false,
            account_reconciliation_required: false,
            account_error: None,
            account_refresh_backoff_until_ms: None,
            account_refresh_retry_due_ms: None,
            all_mids: HashMap::new(),
            all_mids_updated_at_ms: HashMap::new(),
            live_watchlist_flashes: HashMap::new(),
            close_menu_coin: None,
            nuke_confirmation: None,
            pending_nuke_execution: None,
            next_nuke_execution_id: 1,
            positions_sort_column: PositionsSortColumn::Value,
            positions_sort_direction: config::SortDirection::Descending,
            hidden_positions_by_account: cfg
                .hidden_positions_by_account
                .iter()
                .filter_map(|(account, coins)| {
                    let hidden: HashSet<String> = coins
                        .iter()
                        .filter(|coin| !coin.trim().is_empty())
                        .cloned()
                        .collect();
                    (!hidden.is_empty()).then_some((account.clone(), hidden))
                })
                .collect(),
            show_hidden_positions: false,
            chase_orders: BTreeMap::new(),
            chase_spot_symbol_identities: HashMap::new(),
            selected_chase_id: None,
            next_chase_id: 1,
            twap_orders: BTreeMap::new(),
            twap_spot_symbol_identities: HashMap::new(),
            selected_twap_id: None,
            next_twap_id: 1,
            twap_form: crate::twap_state::TwapOrderForm::default(),
            advanced_order_history,
            advanced_order_history_windows: HashMap::new(),
            last_advanced_exchange_request_at: None,
            hide_pnl: cfg.hide_pnl,
            ticker_tape_enabled: cfg.ticker_tape_enabled,
            ticker_tape_scroll_px: 0.0,
            ticker_tape_ctxs: HashMap::new(),
            ticker_tape_contexts_loading: false,
            ticker_tape_contexts_request_id: 0,
            ticker_tape_contexts_request_symbols: Vec::new(),
            ticker_tape_contexts_refresh_pending: false,
            ticker_tape_contexts_last_fetch_ms: None,
            live_watchlists,
            positioning_infos: HashMap::new(),
            next_positioning_info_id: 0,
            positioning_info_pending: HashMap::new(),
            session_data: HashMap::new(),
            next_session_data_id: 0,
            next_session_data_request_id: 0,
            favourite_symbols,
            muted_tickers,
            muted_ticker_input: String::new(),
            muted_ticker_status: None,
            hyperdash_api_key: crate::app_state::sensitive_string(
                cfg.hyperdash_api_key.trim().to_string(),
            ),
            hyperdash_key_generation: 0,
            hyperdash_key_input: crate::app_state::sensitive_string(
                cfg.hyperdash_api_key.trim().to_string(),
            ),
            openrouter_api_key: crate::app_state::sensitive_string(
                cfg.openrouter_api_key.trim().to_string(),
            ),
            openrouter_key_generation: 0,
            openrouter_key_input: crate::app_state::sensitive_string(
                cfg.openrouter_api_key.trim().to_string(),
            ),
            openrouter_key_status: None,
            openrouter_model: cfg.openrouter_model.trim().to_string(),
            toasts: Vec::new(),
            next_toast_id: 0,
            toast_position: cfg.toast_position,
            toast_animations_enabled: cfg.toast_animations_enabled,
            sound_enabled: cfg.sound_enabled,
            desktop_notifications: cfg.desktop_notifications,
            income_alerts_enabled: cfg.income_alerts_enabled,
            last_income_alert_time: None,
            liquidation_alerts_enabled: cfg.liquidation_alerts_enabled,
            liquidation_alert_threshold: cfg.liquidation_alert_threshold,
            liquidation_alert_input: cfg.liquidation_alert_threshold.to_string(),
            market_slippage_pct: cfg.market_slippage_pct,
            market_slippage_input: cfg.market_slippage_pct.to_string(),
            optimistic_account_updates: cfg.optimistic_account_updates,
            hydromancer_realtime_position_pnl_enabled: cfg
                .hydromancer_realtime_position_pnl_enabled,
            tracked_trade_alerts_enabled: cfg.tracked_trade_alerts_enabled,
            main_window_id: None,
            settings_window_id: None,
            add_account_window: None,
            screener: crate::screener_state::ScreenerState::default(),
            chart_screenshot_window_id: None,
            pnl_card_windows: HashMap::new(),
            detached_chart_windows: HashMap::new(),
            chart_screenshot: None,
            chart_screenshot_error: None,
            chart_screenshot_capture_in_progress: false,
            chart_screenshot_next_request_id: 0,
            chart_screenshot_pending_capture: None,
            chart_screenshot_settings: cfg.chart_screenshot_settings.clone(),
            chart_screenshot_menu_open: None,
            chart_surface_active_tools: HashMap::new(),
            chart_surface_viewports: HashMap::new(),
            chart_quick_order_surface: HashMap::new(),
            main_window_size: cfg
                .main_window_width
                .zip(cfg.main_window_height)
                .map(|(w, h)| iced::Size::new(w, h)),
            main_window_pos: cfg
                .main_window_x
                .zip(cfg.main_window_y)
                .map(|(x, y)| iced::Point::new(x, y)),
            wallet_tracker: parts.wallet_tracker,
            wallet_clusters: WalletClusterState::from_config(&cfg.wallet_clusters),
            wallet_detail_windows: HashMap::new(),
            address_book: parts.address_book,
            hovered_wallet_address_actions: None,
            portfolio: PortfolioState::default(),
            income: IncomeState::default(),
            settings_active_tab: SettingsTab::Themes,
            settings_theme_page: ThemeSettingsPage::Overview,
            custom_themes: cfg.custom_themes.clone(),
            journal: {
                let mut journal = journal::JournalState::new_for_account(
                    boot_account.journal_account_key,
                    cfg.journal_entries_by_account.clone(),
                    cfg.journal_entries.clone(),
                );
                journal.width = cfg
                    .journal_window_width
                    .filter(|width| width.is_finite() && *width > 0.0)
                    .unwrap_or(journal::DEFAULT_JOURNAL_WINDOW_WIDTH);
                journal.height = cfg
                    .journal_window_height
                    .filter(|height| height.is_finite() && *height > 0.0)
                    .unwrap_or(journal::DEFAULT_JOURNAL_WINDOW_HEIGHT);
                journal
            },
            spinner_phase: 0.0,
            onboarding_phase: 0.0,
            status_bar_now_ms: Self::now_ms(),
            status_bar_now: Instant::now(),
            candle_data_cache: HashMap::new(),
            candle_data_cache_order: VecDeque::new(),
            heatmap_data_cache: HashMap::new(),
            heatmap_data_cache_order: VecDeque::new(),
            heatmap_pending_charts: HashMap::new(),
            sec_earnings_cache: HashMap::new(),
            sec_earnings_cache_order: VecDeque::new(),
            sec_earnings_request_id: 0,
            sec_earnings_pending_request_ids: HashMap::new(),
            sec_earnings_pending_charts: HashMap::new(),
            sec_filing_summary_cache: HashMap::new(),
            sec_filing_summary_cache_order: VecDeque::new(),
            sec_filing_summary_request_id: 0,
            sec_filing_summary_pending_request_ids: HashMap::new(),
            sec_filing_summary_pending_charts: HashMap::new(),
            liquidation_pending_charts: HashMap::new(),
            liquidation_distribution: {
                let symbol = cfg.liquidation_distribution_symbol.trim().to_string();
                crate::liquidations_distribution_state::LiquidationDistributionState {
                    symbol_search_query: symbol.clone(),
                    symbol,
                    ..Default::default()
                }
            },
            telegram_feed: crate::telegram_feed::TelegramFeedState::new(
                &cfg.telegram_feed_channels,
                &cfg.telegram_feed_private_channels,
                cfg.telegram_feed_notifications_enabled,
                cfg.telegram_feed_fast_mode_enabled,
                cfg.telegram_feed_fast_api_id,
                cfg.telegram_feed_include_outcome_markets,
                cfg.telegram_feed_onboarding_dismissed,
            ),
            x_feed: crate::x_feed::XFeedState::new(
                &cfg.x_feeds,
                &cfg.x_access_token,
                &cfg.x_oauth_client_id,
                &cfg.x_refresh_token,
            ),
            schwab: crate::schwab::SchwabState::new(
                &cfg.schwab_client_id,
                &cfg.schwab_client_secret,
                &cfg.schwab_access_token,
                &cfg.schwab_refresh_token,
            ),
            hotkeys: cfg.hotkeys.clone(),
            chart_timeframe_hotkey_prefix: cfg
                .chart_timeframe_hotkey_prefix
                .and_then(Self::normalize_chart_timeframe_hotkey_prefix),
            recording_hotkey_for: None,
            alfred: crate::alfred_state::AlfredState::default(),
        };
        state.sync_chart_display_denominations();
        state.refresh_live_watchlist_row_caches();
        state
    }
}

use crate::account_state::PositionsSortColumn;
use crate::advanced_order_history::prune_advanced_order_history;
use crate::app_state::TradingTerminal;
use crate::calendar_state::{CalendarImpactFilter, CalendarWindowFilter};
use crate::chart_state::{ChartId, ChartInstance};
use crate::config::{self, KeroseneConfig};
use crate::journal;
use crate::market_state::{SymbolSearchMarketFilter, SymbolSearchSortMode};
use crate::pane_management::AddWidgetPlacement;
use crate::pane_state::PaneKind;
use crate::portfolio_state::{IncomeState, PortfolioState};
use crate::settings_state::SettingsTab;
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiChartInstance};
use crate::wallet_state::{AddressBookEntry, WalletTrackerState};

use iced::widget::pane_grid;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

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
        let mut advanced_order_history = VecDeque::from(cfg.advanced_order_history.clone());
        prune_advanced_order_history(&mut advanced_order_history);

        let mut state = Self {
            saved_layouts: cfg.saved_layouts.clone(),
            active_layout_name: cfg.active_layout_name.clone(),
            layout_input: String::new(),
            live_watchlist_ctxs: HashMap::new(),
            live_watchlist_history: HashMap::new(),
            live_watchlist_contexts_loading: false,
            live_watchlist_history_loading: false,
            live_watchlist_contexts_last_fetch_ms: None,
            live_watchlist_history_loaded_at: HashMap::new(),
            live_watchlist_status: None,
            panes: pane_grid::State::with_configuration(parts.pane_config),
            active_theme: cfg.active_theme.clone(),
            focus: None,
            order_price: String::new(),
            order_quantity: String::new(),
            order_quantity_is_usd: false,
            order_percentage: 0.0,
            order_kind: boot_symbols.order_kind,
            order_reduce_only: cfg.reduce_only,
            order_status: None,
            pending_order_action: None,
            pending_move_order_contexts: HashMap::new(),
            order_presets: cfg.order_presets.clone(),
            preset_is_usd: cfg.preset_is_usd,
            presets_menu_expanded: false,
            preset_edit_mode: false,
            preset_edit_buffer: String::new(),
            preset_edit_idx: None,
            charts: parts.charts,
            next_chart_id: parts.next_chart_id,
            primary_chart_id: None,
            spaghetti_charts: parts.spaghetti_charts,
            next_spaghetti_id: parts.next_spaghetti_id,
            add_widget_menu_open: false,
            add_widget_placement: AddWidgetPlacement::Below,
            account_picker_open: false,
            calendar_events: Vec::new(),
            calendar_error: None,
            calendar_last_fetch: None,
            calendar_loading: false,
            calendar_retry_attempts: 0,
            calendar_next_retry: None,
            calendar_impact_filter: CalendarImpactFilter::MediumHigh,
            calendar_window_filter: CalendarWindowFilter::Upcoming,
            active_symbol: boot_symbols.active_symbol,
            active_symbol_display: boot_symbols.active_symbol_display,
            exchange_symbols: Vec::new(),
            symbols_loading: true,
            symbol_search_query: String::new(),
            symbol_search_sort_mode: SymbolSearchSortMode::from_config_str(
                &cfg.symbol_search_sort_mode,
            ),
            symbol_search_market_filter: SymbolSearchMarketFilter::default(),
            symbol_search_hip3_dex_filter: None,
            symbol_search_result_indices: Vec::new(),
            symbol_search_favourite_count: 0,
            symbol_search_ctxs: HashMap::new(),
            symbol_search_contexts_loading: false,
            symbol_search_contexts_last_fetch_ms: None,
            symbol_search_status: None,
            order_books: HashMap::new(),
            next_order_book_id: 0,
            accounts: cfg.accounts.clone(),
            active_account_index: boot_account.active_account_index,
            ghost_account_secret_ids: HashSet::new(),
            last_persisted_active_account_secret_id: boot_account.last_persisted_secret_id,
            wallet_key_input: boot_account.agent_key,
            wallet_address_input: boot_account.wallet_address,
            hydromancer_api_key: boot_account.hydromancer_key.clone(),
            hydromancer_key_input: boot_account.hydromancer_key,
            secret_store_status: parts.initial_secret_store_status,
            secret_storage_mode: cfg.credential_storage_mode,
            secret_storage_selection: cfg.credential_storage_mode,
            encrypted_secrets: cfg.encrypted_secrets.clone(),
            encrypted_secret_password: crate::app_state::sensitive_string(String::new()),
            encrypted_secret_confirm: crate::app_state::sensitive_string(String::new()),
            encrypted_secrets_unlocked: false,
            show_unlock_credentials_popup: boot_account.show_unlock_credentials_popup,
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
            liquidation_feed_aggregation_enabled: cfg.liquidation_feed_aggregation_enabled,
            liquidation_chart_enabled: false,
            liquidation_summary_enabled: true,
            liquidation_chart_buckets: BTreeMap::new(),
            connected_address: None,
            account_data: None,
            account_loading: boot_account.has_wallet,
            account_error: None,
            all_mids: HashMap::new(),
            all_mids_updated_at_ms: HashMap::new(),
            live_watchlist_flashes: HashMap::new(),
            close_menu_coin: None,
            nuke_confirmation: None,
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
            selected_chase_id: None,
            next_chase_id: 1,
            twap_orders: BTreeMap::new(),
            selected_twap_id: None,
            next_twap_id: 1,
            twap_form: crate::twap_state::TwapOrderForm::default(),
            advanced_order_history,
            advanced_order_history_windows: HashMap::new(),
            last_advanced_exchange_request_at: None,
            hide_pnl: cfg.hide_pnl,
            live_watchlists,
            favourite_symbols,
            muted_tickers,
            muted_ticker_input: String::new(),
            muted_ticker_status: None,
            hyperdash_api_key: crate::app_state::sensitive_string(
                cfg.hyperdash_api_key.trim().to_string(),
            ),
            hyperdash_key_input: crate::app_state::sensitive_string(
                cfg.hyperdash_api_key.trim().to_string(),
            ),
            toasts: Vec::new(),
            next_toast_id: 0,
            sound_enabled: cfg.sound_enabled,
            desktop_notifications: cfg.desktop_notifications,
            income_alerts_enabled: cfg.income_alerts_enabled,
            last_income_alert_time: None,
            liquidation_alerts_enabled: cfg.liquidation_alerts_enabled,
            liquidation_alert_threshold: cfg.liquidation_alert_threshold,
            liquidation_alert_input: cfg.liquidation_alert_threshold.to_string(),
            market_slippage_pct: cfg.market_slippage_pct,
            market_slippage_input: cfg.market_slippage_pct.to_string(),
            tracked_trade_alerts_enabled: cfg.tracked_trade_alerts_enabled,
            main_window_id: None,
            settings_window_id: None,
            chart_screenshot_window_id: None,
            pnl_card_windows: HashMap::new(),
            chart_screenshot: None,
            chart_screenshot_error: None,
            chart_screenshot_capture_in_progress: false,
            chart_screenshot_next_request_id: 0,
            chart_screenshot_pending_request_id: None,
            chart_screenshot_settings: cfg.chart_screenshot_settings.clone(),
            chart_screenshot_menu_open: None,
            main_window_size: cfg
                .main_window_width
                .zip(cfg.main_window_height)
                .map(|(w, h)| iced::Size::new(w, h)),
            main_window_pos: cfg
                .main_window_x
                .zip(cfg.main_window_y)
                .map(|(x, y)| iced::Point::new(x, y)),
            wallet_tracker: parts.wallet_tracker,
            wallet_detail_windows: HashMap::new(),
            address_book: parts.address_book,
            portfolio: PortfolioState::default(),
            income: IncomeState::default(),
            settings_active_tab: SettingsTab::Themes,
            custom_themes: cfg.custom_themes.clone(),
            journal: journal::JournalState::new_for_account(
                boot_account.journal_account_key,
                cfg.journal_entries_by_account.clone(),
                cfg.journal_entries.clone(),
            ),
            spinner_phase: 0.0,
            candle_data_cache: HashMap::new(),
            candle_data_cache_order: VecDeque::new(),
            heatmap_data_cache: HashMap::new(),
            heatmap_data_cache_order: VecDeque::new(),
            heatmap_pending_charts: HashMap::new(),
            liquidation_pending_charts: HashMap::new(),
            hotkeys: cfg.hotkeys.clone(),
            recording_hotkey_for: None,
        };
        state.refresh_live_watchlist_row_caches();
        state
    }
}

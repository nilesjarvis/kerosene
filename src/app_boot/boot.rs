use crate::api::fetch_exchange_symbols;
use crate::app_state::TradingTerminal;
use crate::config;
use crate::layout_persistence::LayoutWidgetConfigs;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::wallet_state::WalletTrackerState;

use iced::Task;

use super::state::BootStateParts;

impl TradingTerminal {
    #[cfg(test)]
    pub(crate) fn boot() -> (Self, Task<Message>) {
        let cfg = config::load_config();
        Self::boot_from_config(cfg)
    }

    pub(crate) fn boot_from_config(mut cfg: config::KeroseneConfig) -> (Self, Task<Message>) {
        let config_warnings = config::take_config_warnings();
        let secret_warnings = config::take_secret_warnings();
        let mut persistence_warnings = config_warnings;
        persistence_warnings.extend(secret_warnings);
        let initial_secret_store_status = if persistence_warnings.is_empty() {
            Some(("Secrets are stored in the OS keychain".to_string(), false))
        } else {
            Some((
                format!("Persistence warning: {}", persistence_warnings.join("; ")),
                true,
            ))
        };

        Self::register_last_layout(&mut cfg);

        // Build the initial movable widget layout from saved (or default) ratios.
        //
        //  Horizontal split (top 70% = main, bottom 30% = bottom row)
        //    main -> Vertical split (left 50% = chart, right 50%)
        //              right -> Vertical split (left 55% = orderbook, right 45% = watchlist)
        //    bottom -> Vertical split (left 65% = tabs, right 35% = order entry)
        let layout_ratios = Self::boot_layout_ratios(&cfg);

        let boot_symbols = Self::boot_symbol_selection(&cfg);
        let muted_tickers = boot_symbols.muted_tickers.clone();
        let symbol = boot_symbols.active_symbol.clone();

        let mut boot_tasks = Vec::new();

        let LayoutWidgetConfigs {
            chart_configs,
            spaghetti_configs,
            next_chart_id,
            next_spaghetti_id,
        } = Self::boot_layout_widget_configs(&cfg, &symbol);

        let (charts, chart_tasks) = Self::boot_chart_instances(
            &chart_configs,
            &muted_tickers,
            cfg.chart_backfill_source,
            cfg.hydromancer_api_key.trim().to_string(),
        );
        boot_tasks.extend(chart_tasks);

        let (spaghetti_charts, spaghetti_tasks) = Self::boot_spaghetti_instances(
            &spaghetti_configs,
            &muted_tickers,
            cfg.chart_backfill_source,
            cfg.hydromancer_api_key.trim().to_string(),
        );
        boot_tasks.extend(spaghetti_tasks);

        let detached_chart_ids: std::collections::BTreeSet<_> = cfg
            .detached_chart_windows
            .iter()
            .map(|window| window.chart_id)
            .collect();
        let first_chart_id = charts
            .keys()
            .copied()
            .filter(|id| !detached_chart_ids.contains(id))
            .min()
            .or_else(|| charts.keys().copied().min())
            .unwrap_or(0);

        let default_pane_config =
            Self::default_boot_pane_configuration(first_chart_id, layout_ratios);

        let pane_config = cfg
            .pane_layout
            .as_ref()
            .and_then(Self::pane_layout_to_configuration)
            .unwrap_or(default_pane_config);

        let boot_account = Self::boot_account_profile(&cfg);
        let has_boot_wallet = boot_account.has_wallet;

        // Auto-connect if we have a saved wallet address in the active profile
        let address_book = Self::build_address_book(&cfg);
        let mut wallet_tracker = WalletTrackerState::from_config(&cfg.wallet_tracker);
        let wallet_tracker_added_labels = Self::add_labeled_addresses_to_wallet_tracker(
            &mut wallet_tracker.tracked_addresses,
            &address_book,
        );
        let mut state = Self::boot_state(BootStateParts {
            cfg: &cfg,
            boot_symbols,
            boot_account,
            initial_secret_store_status,
            pane_config,
            charts,
            next_chart_id,
            spaghetti_charts,
            next_spaghetti_id,
            wallet_tracker,
            address_book,
        });

        if !wallet_tracker_added_labels.is_empty() {
            state.persist_config();
        }

        state.ensure_boot_layout_chart_panes(first_chart_id, &detached_chart_ids);
        state.boot_order_book_instances(&cfg, &muted_tickers);
        state.boot_positioning_info_instances(&cfg, &muted_tickers);
        let book_task = state.boot_order_book_tasks();
        let positioning_task = state.boot_positioning_info_tasks();

        let symbols_task = Task::perform(fetch_exchange_symbols(), Message::SymbolsLoaded);

        let connect_task = if has_boot_wallet {
            Task::done(Message::ConnectWallet)
        } else {
            Task::none()
        };

        boot_tasks.push(symbols_task);
        boot_tasks.push(book_task);
        boot_tasks.push(positioning_task);
        boot_tasks.push(connect_task);

        boot_tasks.extend(state.boot_window_tasks(&cfg));

        if state.is_calendar_open() {
            boot_tasks.push(state.request_calendar_refresh(false));
        }
        if state.pane_is_open(|kind| matches!(kind, PaneKind::TelegramFeed)) {
            boot_tasks.push(state.request_telegram_feed_refresh());
        }
        if state.pane_is_open(|kind| matches!(kind, PaneKind::XFeed)) {
            boot_tasks.push(state.request_x_feed_refresh());
        }
        boot_tasks.push(state.request_hype_etfs_boot_refresh());
        boot_tasks.push(state.request_hype_unstaking_queue_boot_refresh());

        boot_tasks.push(state.request_live_watchlist_refresh(true));
        boot_tasks.push(state.request_ticker_tape_context_refresh(true));
        state.apply_chart_theme_colors();
        state.sync_chart_dotted_background();
        state.sync_chart_hollow_candles();
        state.sync_chart_fisheye();
        state.sync_chart_chromatic_aberration();
        state.sync_chart_edge_blur();
        state.sync_chart_crosshair_style();
        state.sync_chart_crosshair_guides();
        state.sync_chart_crosshair_scale();
        state.sync_chart_hud_readout();

        (state, Task::batch(boot_tasks))
    }
}

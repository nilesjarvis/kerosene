use crate::api::fetch_exchange_symbols;
use crate::app_state::TradingTerminal;
use crate::config::{self, load_config};
use crate::layout_persistence::LayoutWidgetConfigs;
use crate::message::Message;
use crate::wallet_state::WalletTrackerState;

use iced::Task;

use super::state::BootStateParts;

impl TradingTerminal {
    pub(crate) fn boot() -> (Self, Task<Message>) {
        let mut cfg = load_config();
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

        // Build the initial layout tree from saved (or default) ratios.
        //
        //  Horizontal split (top 6% = account bar, bottom 94% = rest)
        //    top  -> AccountSummary
        //    bot  -> Horizontal split (top 70% = main, bottom 30% = bottom row)
        //              main -> Vertical split (left 50% = chart, right 50%)
        //                        right -> Vertical split (left 55% = orderbook, right 45% = watchlist)
        //              bottom -> Vertical split (left 65% = tabs, right 35% = order entry)
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

        let (charts, chart_tasks) = Self::boot_chart_instances(&chart_configs, &muted_tickers);
        boot_tasks.extend(chart_tasks);

        let (spaghetti_charts, spaghetti_tasks) =
            Self::boot_spaghetti_instances(&spaghetti_configs, &muted_tickers);
        boot_tasks.extend(spaghetti_tasks);

        let first_chart_id = charts.keys().copied().min().unwrap_or(0);

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

        state.ensure_boot_layout_chart_panes(first_chart_id);
        state.boot_order_book_instances(&cfg, &muted_tickers);
        let book_task = state.boot_order_book_tasks();

        let symbols_task = Task::perform(fetch_exchange_symbols(), Message::SymbolsLoaded);

        let connect_task = if has_boot_wallet {
            Task::done(Message::ConnectWallet)
        } else {
            Task::none()
        };

        boot_tasks.push(symbols_task);
        boot_tasks.push(book_task);
        boot_tasks.push(connect_task);

        boot_tasks.extend(state.boot_window_tasks());

        if state.is_calendar_open() {
            boot_tasks.push(state.request_calendar_refresh(false));
        }

        boot_tasks.push(state.request_live_watchlist_refresh(true));
        state.apply_chart_theme_colors();

        (state, Task::batch(boot_tasks))
    }
}

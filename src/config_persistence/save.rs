use crate::app_state::TradingTerminal;
use crate::config::{self, save_config};
use crate::message::Message;
use iced::Task;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const CONFIG_SAVE_DEBOUNCE: Duration = Duration::from_millis(750);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigSaveCompletionAction {
    None,
    SavePending,
    Exit,
    /// User asked to close, but the final save returned Err. Stay open
    /// so the failure isn't swallowed silently — the recorded error
    /// status (set by `record_config_save_result`) is already visible
    /// in the UI and the user can retry or accept the loss explicitly.
    BlockExitOnError,
}

fn config_save_is_due(due_at: Option<Instant>, now: Instant) -> bool {
    due_at.is_some_and(|due_at| now >= due_at)
}

fn config_save_should_start(due_at: Option<Instant>, in_flight: bool, now: Instant) -> bool {
    !in_flight && config_save_is_due(due_at, now)
}

fn config_save_completion_action(
    exit_requested: bool,
    has_pending_save: bool,
    save_succeeded: bool,
) -> ConfigSaveCompletionAction {
    match (exit_requested, has_pending_save, save_succeeded) {
        // A debounced save is still due — run it before deciding to exit.
        (true, true, _) => ConfigSaveCompletionAction::SavePending,
        // Exit requested + nothing else pending + last save succeeded → exit.
        (true, false, true) => ConfigSaveCompletionAction::Exit,
        // Exit requested + nothing else pending + last save FAILED → block.
        // Persistence carries account layout, muted tickers, hotkeys,
        // order presets, etc.; silently exiting after a failed save would
        // lose those changes without a recovery opportunity.
        (true, false, false) => ConfigSaveCompletionAction::BlockExitOnError,
        (false, _, _) => ConfigSaveCompletionAction::None,
    }
}

async fn save_config_off_thread(config: config::KeroseneConfig) -> Result<(), String> {
    tokio::task::spawn_blocking(move || save_config(&config))
        .await
        .map_err(|e| format!("config save task failed: {e}"))?
}

#[cfg(test)]
mod tests;

impl TradingTerminal {
    /// Request a config save after the debounce window.
    pub(crate) fn persist_config(&mut self) {
        if self.config_cleared_this_session {
            return;
        }
        self.config_save_due_at = Some(Instant::now() + CONFIG_SAVE_DEBOUNCE);
    }

    pub(crate) fn flush_config_save_if_due(&mut self, now: Instant) -> Task<Message> {
        if !config_save_should_start(self.config_save_due_at, self.config_save_in_flight, now) {
            return Task::none();
        }
        self.config_save_due_at = None;
        self.start_config_save()
    }

    pub(crate) fn flush_pending_config_save_and_exit(&mut self) -> Task<Message> {
        self.config_save_exit_requested = true;
        if self.config_cleared_this_session {
            self.config_save_due_at = None;
            self.config_save_in_flight = false;
            self.config_save_exit_requested = false;
            return iced::exit();
        }

        if self.config_save_in_flight {
            return Task::none();
        }

        if self.config_save_due_at.take().is_some() {
            return self.start_config_save();
        }

        self.config_save_exit_requested = false;
        iced::exit()
    }

    pub(crate) fn handle_config_save_result(
        &mut self,
        result: Result<(), String>,
    ) -> Task<Message> {
        self.config_save_in_flight = false;
        let save_succeeded = result.is_ok();
        self.record_config_save_result(result);

        match config_save_completion_action(
            self.config_save_exit_requested,
            self.config_save_due_at.is_some(),
            save_succeeded,
        ) {
            ConfigSaveCompletionAction::SavePending => {
                self.config_save_due_at = None;
                self.start_config_save()
            }
            ConfigSaveCompletionAction::Exit => {
                self.config_save_exit_requested = false;
                iced::exit()
            }
            ConfigSaveCompletionAction::BlockExitOnError => {
                // Clear the exit-requested flag but keep a save due now. A
                // subsequent close re-runs the final save instead of silently
                // discarding the user's latest layout/preferences after the
                // first failed write.
                self.config_save_exit_requested = false;
                self.config_save_due_at = Some(Instant::now());
                self.push_toast(
                    "Config save failed; close again to retry or keep app open.".to_string(),
                    true,
                );
                Task::none()
            }
            ConfigSaveCompletionAction::None => Task::none(),
        }
    }

    fn start_config_save(&mut self) -> Task<Message> {
        if self.config_cleared_this_session {
            self.config_save_due_at = None;
            return Task::none();
        }
        if self.config_save_in_flight {
            return Task::none();
        }

        let config = self.config_snapshot();
        self.config_save_in_flight = true;
        Task::perform(save_config_off_thread(config), Message::ConfigSaved)
    }

    fn record_config_save_result(&mut self, result: Result<(), String>) {
        match result {
            Ok(()) => {
                if self
                    .secret_store_status
                    .as_ref()
                    .is_some_and(|(status, _)| status.starts_with("Config save failed"))
                {
                    self.secret_store_status = Some(("Config saved".to_string(), false));
                }
            }
            Err(e) => {
                let message = format!("Config save failed: {e}");
                eprintln!("{message}");
                self.secret_store_status = Some((message, true));
            }
        }
    }

    /// Build a config snapshot from the current state.
    fn config_snapshot(&self) -> config::KeroseneConfig {
        if self.config_cleared_this_session {
            return config::KeroseneConfig::default();
        }

        let layout_snapshot = self.saved_layout_snapshot("current".to_string());
        let persisted_accounts = self.persisted_accounts_snapshot();
        let active_account_index = self.persisted_active_account_index(&persisted_accounts);
        let hidden_positions_by_account =
            self.persisted_hidden_positions_by_account(&persisted_accounts);
        let journal_entries_by_account = self.persisted_journal_entries_by_account();
        let journal_entries = match self.journal.active_account_key.as_ref() {
            Some(key) if self.ghost_account_secret_ids.contains(key) => HashMap::new(),
            Some(_) => self.journal.entries.clone(),
            None => self.journal.entries.clone(),
        };

        config::KeroseneConfig {
            saved_layouts: self.saved_layouts.clone(),
            active_layout_name: self.active_layout_name.clone(),
            credential_storage_mode: self.secret_storage_mode,
            encrypted_secrets: self.encrypted_secrets.clone(),
            book_tick_size: layout_snapshot.book_tick_size,
            order_books: layout_snapshot.order_books,
            layout_ratios: layout_snapshot.layout_ratios,
            pane_layout: layout_snapshot.pane_layout,
            charts: self.chart_configs_snapshot(),
            detached_chart_windows: self.detached_chart_window_configs_snapshot(),
            active_symbol: layout_snapshot.active_symbol,
            active_timeframe: layout_snapshot.active_timeframe,
            order_kind: layout_snapshot.order_kind,
            reduce_only: layout_snapshot.reduce_only,
            order_quantity_is_usd: self.order_quantity_is_usd,
            ui_scale: self.ui_scale,
            pane_border_thickness: self.pane_border_thickness,
            pane_corner_radius: self.pane_corner_radius,
            symbol_search_sort_mode: self.symbol_search_sort_mode.config_value().to_string(),
            market_universe: self.market_universe.clone().normalized(),
            chart_screenshot_settings: self.chart_screenshot_settings.clone(),
            accounts: persisted_accounts,
            active_account_index,
            agent_key: String::new().into(),
            wallet_address: String::new(),

            main_window_width: self.main_window_size.map(|s| s.width),
            main_window_height: self.main_window_size.map(|s| s.height),
            main_window_x: self.main_window_pos.map(|p| p.x),
            main_window_y: self.main_window_pos.map(|p| p.y),

            live_watchlists: layout_snapshot.live_watchlists,
            positioning_infos: layout_snapshot.positioning_infos,

            ticker_tape_enabled: layout_snapshot.ticker_tape_enabled,
            favourite_symbols: layout_snapshot.favourite_symbols,
            muted_tickers: self.sorted_muted_tickers(),
            hydromancer_api_key: String::new().into(),
            hyperdash_api_key: String::new().into(),
            sound_enabled: layout_snapshot.sound_enabled,
            desktop_notifications: layout_snapshot.desktop_notifications,
            income_alerts_enabled: layout_snapshot.income_alerts_enabled,
            hide_pnl: self.hide_pnl,
            hidden_positions_by_account,
            liquidation_alerts_enabled: layout_snapshot.liquidation_alerts_enabled,
            liquidation_alert_threshold: layout_snapshot.liquidation_alert_threshold,
            market_slippage_pct: layout_snapshot.market_slippage_pct,
            tracked_trade_alerts_enabled: layout_snapshot.tracked_trade_alerts_enabled,
            tracked_trade_aggregation_enabled: layout_snapshot.tracked_trade_aggregation_enabled,
            liquidation_feed_aggregation_enabled: layout_snapshot
                .liquidation_feed_aggregation_enabled,

            spaghetti_charts: layout_snapshot.spaghetti_charts,
            wallet_tracker: self.wallet_tracker.to_config(&self.address_book),
            address_book: self.address_book_config(),
            active_theme: layout_snapshot.active_theme,
            custom_themes: layout_snapshot.custom_themes,
            journal_entries,
            journal_entries_by_account,
            order_presets: layout_snapshot.order_presets,
            advanced_order_history: self.advanced_order_history.iter().cloned().collect(),
            preset_is_usd: layout_snapshot.preset_is_usd,
            hotkeys: self.hotkeys.clone(),
            chart_timeframe_hotkey_prefix: self.chart_timeframe_hotkey_prefix,
        }
    }
}

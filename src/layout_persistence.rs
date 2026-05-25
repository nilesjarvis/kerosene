use crate::app_state::TradingTerminal;
use crate::config::{self, default_market_slippage_pct, normalize_market_slippage_pct};
use crate::message::Message;
use iced::Task;

mod instances;
mod live_watchlists;
mod order_books;
mod panes;
mod positioning_info;
mod snapshots;
mod widget_configs;

pub(crate) use widget_configs::LayoutWidgetConfigs;

// ---------------------------------------------------------------------------
// Layout persistence
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn apply_layout(&mut self, layout: config::SavedLayout) -> Task<Message> {
        let mut boot_tasks = Vec::new();

        let order_kind = crate::signing::OrderKind::from_config_str(&layout.order_kind);
        let requested_symbol = if layout.active_symbol.is_empty() {
            "HYPE".to_string()
        } else {
            layout.active_symbol.clone()
        };
        let symbol = self
            .restored_active_symbol_key(&requested_symbol)
            .unwrap_or_else(|| "HYPE".to_string());
        let display = self.display_name_for_symbol(&symbol);

        self.order_kind = order_kind;
        self.apply_active_symbol_selection(symbol.clone(), display);
        self.active_theme = layout.active_theme.clone();
        self.order_reduce_only = layout.reduce_only;
        self.order_presets = layout.order_presets.clone();
        self.preset_is_usd = layout.preset_is_usd;
        self.ticker_tape_enabled = layout.ticker_tape_enabled;
        self.ticker_tape_scroll_px = 0.0;
        self.favourite_symbols = layout
            .favourite_symbols
            .iter()
            .filter(|symbol| !self.symbol_key_is_hidden(symbol))
            .cloned()
            .collect();
        self.sound_enabled = layout.sound_enabled;
        self.desktop_notifications = layout.desktop_notifications;
        self.income_alerts_enabled = layout.income_alerts_enabled;
        self.liquidation_alerts_enabled = layout.liquidation_alerts_enabled;
        self.liquidation_alert_threshold = layout.liquidation_alert_threshold;
        self.liquidation_alert_input = layout.liquidation_alert_threshold.to_string();
        let market_slippage_pct = normalize_market_slippage_pct(layout.market_slippage_pct)
            .unwrap_or_else(default_market_slippage_pct);
        self.market_slippage_pct = market_slippage_pct;
        self.market_slippage_input = market_slippage_pct.to_string();
        self.tracked_trade_alerts_enabled = layout.tracked_trade_alerts_enabled;
        self.tracked_trade_aggregation_enabled = layout.tracked_trade_aggregation_enabled;
        self.liquidation_feed_aggregation_enabled = layout.liquidation_feed_aggregation_enabled;
        self.custom_themes = layout.custom_themes.clone();

        let widget_configs = self.normalized_layout_widget_configs(&layout);
        let chart_configs = widget_configs.chart_configs;
        let spaghetti_configs = widget_configs.spaghetti_configs;
        let next_chart_id = widget_configs.next_chart_id;
        let next_spaghetti_id = widget_configs.next_spaghetti_id;

        boot_tasks.extend(self.restore_layout_chart_instances(
            &chart_configs,
            &spaghetti_configs,
            next_chart_id,
            next_spaghetti_id,
        ));
        boot_tasks.extend(self.close_detached_windows_for_missing_charts());
        self.prune_chart_surface_state();

        self.restore_layout_panes(&layout);
        boot_tasks.push(self.restore_layout_order_books(&layout));
        self.restore_layout_live_watchlists(&layout);
        boot_tasks.push(self.restore_layout_positioning_infos(&layout));

        if self.is_calendar_open() {
            boot_tasks.push(self.request_calendar_refresh(false));
        }

        boot_tasks.push(self.request_ticker_tape_context_refresh(true));
        boot_tasks.push(self.request_live_watchlist_refresh(true));
        boot_tasks.push(self.sync_main_window_min_size());
        self.apply_chart_theme_colors();
        self.sync_chart_dotted_background();

        Task::batch(boot_tasks)
    }

    fn close_detached_windows_for_missing_charts(&mut self) -> Vec<Task<Message>> {
        let stale_window_ids: Vec<_> = self
            .detached_chart_windows
            .iter()
            .filter_map(|(window_id, state)| {
                (!self.charts.contains_key(&state.chart_id)).then_some(*window_id)
            })
            .collect();

        for window_id in &stale_window_ids {
            self.remove_detached_chart_window_state(*window_id);
        }

        stale_window_ids
            .into_iter()
            .map(iced::window::close)
            .collect()
    }
}

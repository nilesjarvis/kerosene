use crate::app_state::TradingTerminal;
use crate::config::{self, default_market_slippage_pct, normalize_market_slippage_pct};
use crate::message::Message;
use iced::Task;

mod instances;
mod live_watchlists;
mod order_books;
mod panes;
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
        let mut symbol = if layout.active_symbol.is_empty() {
            "HYPE".to_string()
        } else {
            layout.active_symbol.clone()
        };
        if self.symbol_key_is_hidden(&symbol)
            && let Some(fallback) = self.fallback_unmuted_symbol_key()
        {
            symbol = fallback;
        }
        let display = symbol.split(':').nth(1).unwrap_or(&symbol).to_string();

        self.order_kind = order_kind;
        self.active_symbol = symbol.clone();
        self.active_symbol_display = display;
        self.active_theme = layout.active_theme.clone();
        self.order_reduce_only = layout.reduce_only;
        self.order_presets = layout.order_presets.clone();
        self.preset_is_usd = layout.preset_is_usd;
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

        self.restore_layout_panes(&layout);
        boot_tasks.push(self.restore_layout_order_books(&layout));
        self.restore_layout_live_watchlists(&layout);

        if self.is_calendar_open() {
            boot_tasks.push(self.request_calendar_refresh(false));
        }

        boot_tasks.push(self.request_live_watchlist_refresh(true));
        self.apply_chart_theme_colors();

        Task::batch(boot_tasks)
    }
}

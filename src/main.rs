#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod account;
mod account_analytics;
mod account_metrics;
mod account_positions;
mod account_state;
mod account_update;
mod account_views;
mod add_widget_menu;
mod advanced_order_history;
mod alfred_state;
mod alfred_update;
mod alfred_views;
mod annotation_update;
mod annotations;
mod api;
mod app_boot;
mod app_fonts;
mod app_state;
mod app_theme;
mod app_time;
mod app_update;
mod calendar_state;
mod calendar_update;
mod calendar_views;
mod chart;
mod chart_background;
mod chart_screenshot;
mod chart_state;
mod chart_update;
mod chart_views;
mod chrome_update;
mod config;
mod config_persistence;
mod denomination;
mod feed_state;
mod feed_update;
mod feed_views;
mod helpers;
mod hotkey_state;
mod hydromancer_api;
mod hype_etf_state;
mod hyperdash_api;
mod hyperdash_update;
mod journal;
mod journal_update;
mod journal_views;
mod layout_persistence;
mod layout_preview;
mod layout_update;
mod loading_views;
mod main_view;
mod market_state;
mod market_update;
mod market_views;
mod message;
mod notification_state;
mod order_execution;
mod order_pending_indicators;
mod order_update;
mod order_views;
mod pane_interaction_update;
mod pane_management;
mod pane_state;
mod pane_update;
mod pnl_card;
mod portfolio_state;
mod portfolio_update;
mod positioning_state;
mod preferences_update;
mod risk_state;
mod secret_storage;
mod settings_state;
mod settings_update;
mod settings_views;
mod signing;
mod sound;
mod spaghetti;
mod spaghetti_state;
mod spaghetti_update;
mod spaghetti_views;
mod spread_chart;
mod status_bar;
mod subscription_state;
mod timeframe;
mod toast_overlay;
mod twap_state;
mod wallet_state;
mod wallet_update;
mod wallet_views;
mod window_chrome;
mod window_update;
mod ws;

use app_state::TradingTerminal;

#[cfg(test)]
mod positions_funding_tests;

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

pub fn main() -> iced::Result {
    let config = config::load_config();
    let settings = app_fonts::settings_from_config(&config);

    iced::daemon(
        move || TradingTerminal::boot_from_config(config.clone()),
        TradingTerminal::update,
        TradingTerminal::view_window,
    )
    .settings(settings)
    .subscription(TradingTerminal::subscription)
    .title(TradingTerminal::window_title)
    .theme(TradingTerminal::window_theme)
    .scale_factor(TradingTerminal::window_scale_factor)
    .run()
}

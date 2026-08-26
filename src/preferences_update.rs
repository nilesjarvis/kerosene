use crate::app_state::TradingTerminal;
use crate::config::{
    normalize_alfred_popup_scale, normalize_chart_chromatic_aberration_strength,
    normalize_chart_crosshair_scale, normalize_chart_dotted_background_opacity,
    normalize_chart_edge_blur_strength, normalize_chart_fisheye_strength,
    normalize_chart_gradient_contrast, normalize_market_slippage_pct,
    normalize_pane_border_thickness, normalize_pane_corner_radius, normalize_ui_scale,
    normalize_window_background_opacity,
};
use crate::helpers::path_neutral_io_error_detail;
use crate::market_state::SymbolSearchMarketFilter;
use crate::message::Message;
use iced::Task;
#[cfg(target_os = "linux")]
use iced::window;
use std::path::Path;

mod fonts;
mod hotkeys;
mod muted_tickers;
mod sounds;

const IMPORT_MIB: u64 = 1024 * 1024;
pub(super) const MAX_IMPORTED_FONT_BYTES: u64 = 64 * IMPORT_MIB;
pub(super) const MAX_IMPORTED_HUD_SOUND_BYTES: u64 = 16 * IMPORT_MIB;

pub(super) fn ensure_import_file_within_limit(
    path: &Path,
    kind: &str,
    max_bytes: u64,
) -> Result<(), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| import_io_failure(&format!("read selected {kind} file metadata"), &e))?;
    if !metadata.is_file() {
        return Err(format!("selected {kind} path is not a file"));
    }

    let len = metadata.len();
    if len > max_bytes {
        return Err(format!(
            "selected {kind} file is too large: {len} bytes (max {} MiB)",
            max_bytes / IMPORT_MIB
        ));
    }

    Ok(())
}

pub(super) fn import_io_failure(action: &str, error: &std::io::Error) -> String {
    format!("{action} failed: {}", path_neutral_io_error_detail(error))
}

impl TradingTerminal {
    pub(crate) fn update_preferences(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ThemeChanged(theme_name) => {
                self.active_theme = theme_name;
                self.apply_chart_theme_colors();
                self.persist_config();
            }
            Message::UiScaleChanged(value) => {
                self.ui_scale = normalize_ui_scale(value);
                self.persist_config();
                return self.sync_main_window_min_size();
            }
            Message::ToggleWindowTransparency(enabled)
                if self.window_transparency_enabled != enabled =>
            {
                self.window_transparency_enabled = enabled;
                self.apply_chart_theme_colors();
                self.persist_config();
            }
            Message::ToggleWindowBackgroundBlur(enabled)
                if self.window_background_blur_enabled != enabled =>
            {
                self.window_background_blur_enabled = enabled;
                self.persist_config();
                let status = if crate::window_chrome::background_blur_supported() {
                    "Restart Kerosene to apply background blur to existing windows."
                } else {
                    crate::window_chrome::background_blur_unavailable_reason()
                };
                self.push_toast(status.to_string(), false);
            }
            Message::WindowBackgroundOpacityChanged(value) => {
                let opacity = normalize_window_background_opacity(value);
                if (self.window_background_opacity - opacity).abs() > f32::EPSILON {
                    self.window_background_opacity = opacity;
                    self.apply_chart_theme_colors();
                    self.persist_config();
                }
            }
            Message::ToggleChartDottedBackground(enabled)
                if self.chart_dotted_background != enabled =>
            {
                self.chart_dotted_background = enabled;
                self.sync_chart_dotted_background();
                self.persist_config();
            }
            Message::ChartDottedBackgroundOpacityChanged(value) => {
                let opacity = normalize_chart_dotted_background_opacity(value);
                if (self.chart_dotted_background_opacity - opacity).abs() > f32::EPSILON {
                    self.chart_dotted_background_opacity = opacity;
                    self.sync_chart_dotted_background();
                    self.persist_config();
                }
            }
            Message::ToggleChartGradientBackground(enabled)
                if self.chart_gradient_background != enabled =>
            {
                self.chart_gradient_background = enabled;
                self.sync_chart_gradient_background();
                self.persist_config();
            }
            Message::ChartGradientContrastChanged(value) => {
                let contrast = normalize_chart_gradient_contrast(value);
                if (self.chart_gradient_contrast - contrast).abs() > f32::EPSILON {
                    self.chart_gradient_contrast = contrast;
                    self.sync_chart_gradient_background();
                    self.persist_config();
                }
            }
            Message::ChartHollowCandleModeChanged(mode)
                if self.chart_hollow_candle_mode != mode =>
            {
                self.chart_hollow_candle_mode = mode;
                self.sync_chart_hollow_candles();
                self.persist_config();
            }
            Message::ChartSeriesStyleChanged(style) if self.chart_series_style != style => {
                self.chart_series_style = style;
                self.sync_chart_series_style();
                self.persist_config();
            }
            Message::ToggleChartFisheye(enabled) if self.chart_fisheye_enabled != enabled => {
                self.chart_fisheye_enabled = enabled;
                self.sync_chart_fisheye();
                self.persist_config();
            }
            Message::ChartFisheyeStrengthChanged(value) => {
                let strength = normalize_chart_fisheye_strength(value);
                if (self.chart_fisheye_strength - strength).abs() > f32::EPSILON {
                    self.chart_fisheye_strength = strength;
                    self.sync_chart_fisheye();
                    self.persist_config();
                }
            }
            Message::ToggleChartChromaticAberration(enabled)
                if self.chart_chromatic_aberration_enabled != enabled =>
            {
                self.chart_chromatic_aberration_enabled = enabled;
                self.sync_chart_chromatic_aberration();
                self.persist_config();
            }
            Message::ChartChromaticAberrationStrengthChanged(value) => {
                let strength = normalize_chart_chromatic_aberration_strength(value);
                if (self.chart_chromatic_aberration_strength - strength).abs() > f32::EPSILON {
                    self.chart_chromatic_aberration_strength = strength;
                    self.sync_chart_chromatic_aberration();
                    self.persist_config();
                }
            }
            Message::ToggleChartEdgeBlur(enabled) if self.chart_edge_blur_enabled != enabled => {
                self.chart_edge_blur_enabled = enabled;
                self.sync_chart_edge_blur();
                self.persist_config();
            }
            Message::ChartEdgeBlurStrengthChanged(value) => {
                let strength = normalize_chart_edge_blur_strength(value);
                if (self.chart_edge_blur_strength - strength).abs() > f32::EPSILON {
                    self.chart_edge_blur_strength = strength;
                    self.sync_chart_edge_blur();
                    self.persist_config();
                }
            }
            Message::ChartCrosshairStyleChanged(style)
                if self.chart_crosshair_style != style.normalized() =>
            {
                let style = style.normalized();
                self.chart_crosshair_style = style;
                self.sync_chart_crosshair_style();
                self.persist_config();
            }
            Message::ToggleChartCrosshairGuides(enabled)
                if self.chart_crosshair_guides_enabled != enabled =>
            {
                self.chart_crosshair_guides_enabled = enabled;
                self.sync_chart_crosshair_guides();
                self.persist_config();
            }
            Message::ChartCrosshairScaleChanged(value) => {
                let scale = normalize_chart_crosshair_scale(value);
                if (self.chart_crosshair_scale - scale).abs() > f32::EPSILON {
                    self.chart_crosshair_scale = scale;
                    self.sync_chart_crosshair_scale();
                    self.persist_config();
                }
            }
            Message::ToastPositionChanged(position) if self.toast_position != position => {
                self.toast_position = position;
                self.persist_config();
            }
            Message::ToggleToastAnimations(enabled) if self.toast_animations_enabled != enabled => {
                self.toast_animations_enabled = enabled;
                self.persist_config();
            }
            Message::ToggleOptimisticAccountUpdates(enabled)
                if self.optimistic_account_updates != enabled =>
            {
                self.optimistic_account_updates = enabled;
                self.persist_config();
            }
            Message::ToggleHydromancerRealtimePositionPnl(enabled)
                if self.hydromancer_realtime_position_pnl_enabled != enabled =>
            {
                self.hydromancer_realtime_position_pnl_enabled = enabled;
                self.persist_config();
                if enabled && self.hydromancer_api_key.trim().is_empty() {
                    self.push_toast(
                        "Real-time position PnL will start after a Hydromancer API key is saved"
                            .to_string(),
                        true,
                    );
                }
            }
            Message::ChartHudReadoutToggled(element, enabled)
                if self.chart_hud_readout.enabled(element) != enabled =>
            {
                self.chart_hud_readout.set(element, enabled);
                self.sync_chart_hud_readout();
                self.persist_config();
            }
            message @ (Message::ChartHudOrderSoundChanged(_)
            | Message::ChartHudOrderSoundVolumeChanged(_)
            | Message::ImportChartHudOrderSound
            | Message::ChartHudOrderSoundImported(_)
            | Message::TestChartHudOrderSound
            | Message::ToggleChartHudUiSounds(_)) => {
                return self.update_sound_preferences(message);
            }
            Message::ReadDataProviderChanged(provider) if self.read_data_provider != provider => {
                self.read_data_provider = provider;
                self.bump_read_data_provider_generation();
                self.chart_backfill_source = provider.chart_backfill_source();
                self.invalidate_portfolio_income_refreshes();
                self.invalidate_wallet_read_data_requests();
                self.journal.clear_snapshot_cache();
                self.journal.snapshot_requests.clear();
                self.journal.expanded_snapshot_trade_ids.clear();
                self.persist_config();
                if provider == crate::config::ReadDataProvider::Hydromancer
                    && self.hydromancer_api_key.trim().is_empty()
                {
                    self.push_toast(
                        "Hydromancer selected; read data will use Hyperliquid until an API key is saved"
                            .to_string(),
                        true,
                    );
                } else {
                    self.push_toast(
                        format!("Read data provider set to {}", provider.label()),
                        false,
                    );
                }
                return Task::batch([
                    self.reload_chart_backfills_for_source_change(),
                    self.refresh_account_data(),
                ]);
            }
            Message::AlfredPopupScaleChanged(value) => {
                self.alfred_popup_scale = normalize_alfred_popup_scale(value);
                self.persist_config();
            }
            message @ (Message::DisplayFontChanged(_)
            | Message::MonospaceFontChanged(_)
            | Message::ImportDisplayFont
            | Message::DisplayFontImported(_)
            | Message::ImportMonospaceFont
            | Message::MonospaceFontImported(_)) => {
                return self.update_font_preferences(message);
            }
            Message::PaneBorderThicknessChanged(value) => {
                self.pane_border_thickness = normalize_pane_border_thickness(value);
                self.persist_config();
                return self.sync_main_window_min_size();
            }
            Message::PaneCornerRadiusChanged(value) => {
                self.pane_corner_radius = normalize_pane_corner_radius(value);
                self.persist_config();
            }
            Message::ToggleOuterWidgetBorder(enabled)
                if self.outer_widget_border_enabled != enabled =>
            {
                self.outer_widget_border_enabled = enabled;
                self.persist_config();
                return self.sync_main_window_min_size();
            }
            Message::DefaultWidgetPaddingChanged(value) => {
                let before = self.widget_padding_default;
                self.set_default_widget_padding(value);
                if (self.widget_padding_default - before).abs() > f32::EPSILON {
                    self.persist_config();
                    return self.sync_main_window_min_size();
                }
            }
            Message::FocusedWidgetPaddingChanged(value) => {
                let before = self.focused_widget_padding();
                if self.set_focused_widget_padding(value) && self.focused_widget_padding() != before
                {
                    self.persist_config();
                    return self.sync_main_window_min_size();
                }
            }
            Message::ResetFocusedWidgetPadding if self.reset_focused_widget_padding() => {
                self.persist_config();
                return self.sync_main_window_min_size();
            }
            Message::ToggleCustomWindowChrome(enabled)
                if self.custom_window_chrome_enabled != enabled =>
            {
                if !crate::window_chrome::custom_chrome_supported() {
                    return Task::none();
                }
                self.custom_window_chrome_enabled = enabled;
                self.persist_config();

                #[cfg(target_os = "linux")]
                {
                    self.custom_window_chrome_active = enabled;
                    return Task::batch([
                        self.sync_open_window_decorations(),
                        self.sync_main_window_min_size(),
                    ]);
                }

                #[cfg(target_os = "macos")]
                {
                    self.push_toast(
                        "Restart Kerosene to apply the OS bar preference.".to_string(),
                        false,
                    );
                }
            }
            Message::MutedTickerInputChanged(value) => {
                self.muted_ticker_input = value;
                self.muted_ticker_status = None;
            }
            message @ (Message::MuteTicker | Message::UnmuteTicker(_)) => {
                return self.update_muted_ticker_preferences(message);
            }
            Message::MarketUniverseChanged(universe) => {
                let universe = self.normalize_market_universe_selection(universe);
                if self.market_universe == universe {
                    return Task::none();
                }

                let status = match universe.selected_hip3_dex() {
                    Some(dex) => {
                        self.symbol_search_market_filter = SymbolSearchMarketFilter::Hip3;
                        self.symbol_search_hip3_dex_filter = Some(dex.to_string());
                        format!("Showing HIP-3 exchange {dex} only")
                    }
                    None => {
                        self.symbol_search_market_filter = SymbolSearchMarketFilter::All;
                        self.symbol_search_hip3_dex_filter = None;
                        "Showing all markets".to_string()
                    }
                };

                self.market_universe = universe;
                self.clear_percentage_order_quantity();
                self.muted_ticker_status = Some((status.clone(), false));
                self.push_toast(status, false);
                let hidden_chase_ids: Vec<u64> = self
                    .chase_orders
                    .iter()
                    .filter_map(|(id, chase)| self.symbol_key_is_hidden(&chase.coin).then_some(*id))
                    .collect();
                let stop_chase_task = Task::batch(hidden_chase_ids.into_iter().map(|id| {
                    self.stop_chase_by_id_with_reason(
                        id,
                        "Chase stopped: ticker was hidden by market universe",
                        false,
                    )
                }));
                let hidden_twap_ids: Vec<u64> = self
                    .twap_orders
                    .iter()
                    .filter_map(|(id, twap)| {
                        (!twap.status.is_terminal() && self.symbol_key_is_hidden(&twap.coin))
                            .then_some(*id)
                    })
                    .collect();
                let stop_twap_task = Task::batch(hidden_twap_ids.into_iter().map(|id| {
                    self.stop_twap_with_reason(
                        id,
                        "TWAP stopped: ticker was hidden by market universe",
                        false,
                    )
                }));
                let reconcile_task = self.reconcile_market_universe_state();
                self.refresh_symbol_search_results();
                self.refresh_live_watchlist_row_caches();
                self.persist_config();
                let account_task = self.refresh_account_data();
                return Task::batch([
                    stop_chase_task,
                    stop_twap_task,
                    reconcile_task,
                    self.request_symbol_search_context_refresh(true),
                    self.request_live_watchlist_refresh(true),
                    self.request_screener_data_refresh(true),
                    account_task,
                ]);
            }
            Message::DisplayDenominationChanged(denomination) => {
                let denomination = denomination.normalized();
                if self.display_denomination == denomination {
                    return Task::none();
                }

                self.display_denomination = denomination;
                self.sync_chart_display_denominations();
                self.persist_config();
                let mut tasks = self.mids_bootstrap_tasks();
                tasks.push(self.request_live_watchlist_refresh(true));
                return Task::batch(tasks);
            }
            Message::MarketSlippageInputChanged(value) => {
                self.market_slippage_input = value;
            }
            Message::SaveMarketSlippage => {
                match self
                    .market_slippage_input
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .and_then(normalize_market_slippage_pct)
                {
                    Some(value) => {
                        self.market_slippage_pct = value;
                        self.market_slippage_input = value.to_string();
                        self.muted_ticker_status =
                            Some((format!("Market slippage set to {value}%"), false));
                        self.persist_config();
                    }
                    None => {
                        self.market_slippage_input = self.market_slippage_pct.to_string();
                        self.muted_ticker_status = Some((
                            "Market slippage must be between 0% and 20%".to_string(),
                            true,
                        ));
                    }
                }
            }
            message @ (Message::StartRecordingHotkey(_)
            | Message::ClearHotkey(_)
            | Message::KeyboardEvent(_, _, _)
            | Message::ExecuteHotkey(_)) => return self.update_hotkey_preferences(message),
            _ => {}
        }

        Task::none()
    }

    #[cfg(target_os = "linux")]
    fn sync_open_window_decorations(&self) -> Task<Message> {
        Task::batch(
            self.open_window_ids()
                .into_iter()
                .map(window::toggle_decorations),
        )
    }

    #[cfg(target_os = "linux")]
    fn open_window_ids(&self) -> Vec<window::Id> {
        let mut ids = Vec::new();

        if let Some(id) = self.main_window_id {
            ids.push(id);
        }
        if let Some(id) = self.settings_window_id {
            ids.push(id);
        }
        if let Some(id) = self.screener.window_id {
            ids.push(id);
        }
        if let Some(id) = self.chart_screenshot_window_id {
            ids.push(id);
        }
        if let Some(state) = &self.quick_trade_editor {
            ids.push(state.window_id);
        }
        if let Some(id) = self.wallet_tracker.window_id {
            ids.push(id);
        }
        if let Some(id) = self.journal.window_id {
            ids.push(id);
        }

        ids.extend(self.canvases.values().filter_map(|canvas| canvas.window_id));
        ids.extend(self.wallet_detail_windows.keys().copied());
        ids.extend(self.twap_orders.values().filter_map(|twap| twap.window_id));
        ids.extend(self.advanced_order_history_windows.keys().copied());
        ids.extend(self.pnl_card_windows.keys().copied());
        ids.extend(self.detached_chart_windows.keys().copied());
        ids.extend(self.detached_spaghetti_windows.keys().copied());

        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ExchangeSymbol, MarketType};
    use crate::chart_state::ChartInstance;
    use crate::config::MarketUniverseConfig;
    use crate::market_state::{LiveWatchlistInstance, OrderBookInstance, OrderBookSymbolMode};
    use crate::positioning_state::PositioningInfoInstance;
    use crate::session_data_state::{SessionDataInstance, SessionDataLookback};
    use crate::spaghetti::Series;
    use crate::spaghetti_state::SpaghettiChartInstance;
    use crate::timeframe::Timeframe;
    use std::io::{Error, ErrorKind};

    fn perp_symbol(key: &str) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key
                .split_once(':')
                .map_or(key, |(_, ticker)| ticker)
                .to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 2,
            max_leverage: 50,
            only_isolated: false,
            market_type: MarketType::Perp,
            outcome: None,
        }
    }

    #[test]
    fn missing_import_file_metadata_error_redacts_source_path() {
        let path = std::env::temp_dir().join("kerosene-secret-source-font.ttf");

        let error = ensure_import_file_within_limit(&path, "font", MAX_IMPORTED_FONT_BYTES)
            .expect_err("missing import source should fail metadata read");

        assert!(error.contains("read selected font file metadata failed"));
        assert!(error.contains("not found"));
        assert!(!error.contains(&path.display().to_string()));
        assert!(!error.contains("secret-source-font"));
    }

    #[test]
    fn import_io_failure_uses_kind_without_custom_error_payload() {
        let error = Error::new(
            ErrorKind::PermissionDenied,
            "denied /home/alice/secret-font.ttf api_key=font-secret",
        );

        let rendered = import_io_failure("read selected font file", &error);

        assert_eq!(
            rendered,
            "read selected font file failed: permission denied"
        );
        assert!(!rendered.contains("/home/alice"));
        assert!(!rendered.contains("font-secret"));
    }

    #[test]
    fn transparency_preferences_apply_immediately_and_normalize_opacity() {
        let (mut terminal, _) = TradingTerminal::boot();
        assert!(!terminal.window_transparency_enabled);

        let _task = terminal.update_preferences(Message::ToggleWindowTransparency(true));
        assert!(terminal.window_transparency_enabled);
        assert!(terminal.config_save_due_at.is_some());

        terminal.config_save_due_at = None;
        let _task = terminal.update_preferences(Message::WindowBackgroundOpacityChanged(0.1));
        assert_eq!(
            terminal.window_background_opacity,
            crate::config::MIN_WINDOW_BACKGROUND_OPACITY
        );
        assert!(terminal.config_save_due_at.is_some());

        terminal.config_save_due_at = None;
        let _task = terminal.update_preferences(Message::WindowBackgroundOpacityChanged(f32::NAN));
        assert_eq!(
            terminal.window_background_opacity,
            crate::config::DEFAULT_WINDOW_BACKGROUND_OPACITY
        );
        assert!(terminal.config_save_due_at.is_some());
    }

    #[test]
    fn background_blur_preference_persists_and_reports_runtime_support() {
        let (mut terminal, _) = TradingTerminal::boot();
        assert!(!terminal.window_background_blur_enabled);

        let _task = terminal.update_preferences(Message::ToggleWindowBackgroundBlur(true));

        assert!(terminal.window_background_blur_enabled);
        assert!(terminal.config_save_due_at.is_some());
        let expected_status = if crate::window_chrome::background_blur_supported() {
            "Restart Kerosene to apply background blur to existing windows."
        } else {
            crate::window_chrome::background_blur_unavailable_reason()
        };
        assert!(
            terminal
                .toasts
                .iter()
                .any(|toast| toast.message == expected_status)
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn decoration_sync_includes_open_canvas_windows() {
        let (mut terminal, _) = TradingTerminal::boot();
        let canvas_window_id = iced::window::Id::unique();
        terminal.insert_test_canvas_pane(7, crate::pane_state::PaneKind::Watchlist);
        terminal.canvases.get_mut(&7).expect("Canvas").window_id = Some(canvas_window_id);

        assert!(terminal.open_window_ids().contains(&canvas_window_id));
    }

    #[test]
    fn root_update_routes_hud_ui_sound_toggle_to_sound_preferences() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.chart_hud_ui_sounds = true;
        terminal.config_save_due_at = None;

        let _task = terminal.update(Message::ToggleChartHudUiSounds(false));

        assert!(!terminal.chart_hud_ui_sounds);
        assert!(terminal.config_save_due_at.is_some());
    }

    #[test]
    fn market_universe_switch_preserves_favourites_and_comparison_configuration() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.market_universe = MarketUniverseConfig::All;
        terminal.exchange_symbols = vec![
            perp_symbol("HYPE"),
            perp_symbol("BTC"),
            perp_symbol("xyz:NVDA"),
        ];
        terminal.active_symbol = "HYPE".to_string();
        terminal.favourite_symbols = vec!["HYPE".to_string(), "BTC".to_string()];

        terminal.charts.clear();
        let mut chart = ChartInstance::new(7, "HYPE".to_string(), Timeframe::H1);
        chart.set_secondary_symbol_identity("BTC".to_string(), "BTC".to_string());
        terminal.charts.insert(7, chart);

        let mut comparison = SpaghettiChartInstance::new_empty(9);
        comparison.canvas.series.push(Series {
            symbol: "HYPE".to_string(),
            display: "HYPE".to_string(),
            candles: Vec::new(),
            color: iced::Color::WHITE,
            loaded: false,
        });
        terminal.spaghetti_charts.clear();
        terminal.spaghetti_charts.insert(9, comparison);

        terminal.live_watchlists.clear();
        terminal.live_watchlists.insert(
            11,
            LiveWatchlistInstance {
                id: 11,
                symbols: vec!["HYPE".to_string(), "BTC".to_string()],
                search_query: String::new(),
                sort_column: Default::default(),
                sort_direction: Default::default(),
                visible_columns: crate::config::default_live_watchlist_columns(),
                row_cache: Vec::new(),
            },
        );
        terminal.order_books.clear();
        terminal.order_books.insert(
            12,
            OrderBookInstance::new(12, OrderBookSymbolMode::Fixed("HYPE".to_string()), 0.01),
        );
        terminal.positioning_infos.clear();
        terminal
            .positioning_infos
            .insert(13, PositioningInfoInstance::new(13, "HYPE".to_string()));
        terminal.session_data.clear();
        terminal.session_data.insert(
            14,
            SessionDataInstance::new(14, "HYPE".to_string(), SessionDataLookback::FourWeeks),
        );

        let _task = terminal.update_preferences(Message::MarketUniverseChanged(
            MarketUniverseConfig::hip3_dex("xyz"),
        ));

        assert_eq!(terminal.favourite_symbols, ["HYPE", "BTC"]);
        let chart = terminal.charts.get(&7).expect("comparison chart");
        assert_eq!(chart.symbol, "HYPE");
        assert_eq!(chart.secondary_symbol.as_deref(), Some("BTC"));
        assert_eq!(
            terminal.spaghetti_charts[&9].canvas.series[0].symbol,
            "HYPE"
        );
        assert_eq!(terminal.live_watchlists[&11].symbols, ["HYPE", "BTC"]);
        assert_eq!(
            terminal.order_books[&12].mode,
            OrderBookSymbolMode::Fixed("HYPE".to_string())
        );
        assert_eq!(terminal.positioning_infos[&13].symbol, "HYPE");
        assert_eq!(terminal.session_data[&14].symbol, "HYPE");

        // A config save while the narrower market is selected must not erase
        // the temporarily filtered symbols either.
        assert_eq!(terminal.favourite_symbols_config_values(), ["HYPE", "BTC"]);
        let chart_config = terminal
            .chart_configs_snapshot()
            .into_iter()
            .find(|config| config.id == 7)
            .expect("chart config");
        assert_eq!(chart_config.symbol, "HYPE");
        assert_eq!(chart_config.secondary_symbol.as_deref(), Some("BTC"));
        assert_eq!(
            terminal.spaghetti_chart_configs_snapshot()[0].symbols,
            ["HYPE"]
        );
        assert_eq!(
            terminal.live_watchlist_configs_snapshot()[0].symbols,
            ["HYPE", "BTC"]
        );
        assert!(matches!(
            terminal.order_book_configs_snapshot()[0].mode,
            crate::config::OrderBookSymbolModeConfig::Fixed(ref symbol) if symbol == "HYPE"
        ));
        assert_eq!(
            terminal.positioning_info_configs_snapshot()[0].symbol,
            "HYPE"
        );
        assert_eq!(terminal.session_data_configs_snapshot()[0].symbol, "HYPE");

        let _task =
            terminal.update_preferences(Message::MarketUniverseChanged(MarketUniverseConfig::All));
        assert_eq!(terminal.favourite_symbols, ["HYPE", "BTC"]);
        assert_eq!(terminal.charts[&7].secondary_symbol.as_deref(), Some("BTC"));
        assert_eq!(
            terminal.spaghetti_charts[&9].canvas.series[0].symbol,
            "HYPE"
        );
    }
}

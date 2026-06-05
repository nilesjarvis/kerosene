use crate::app_state::TradingTerminal;
use crate::config::{
    normalize_alfred_popup_scale, normalize_chart_chromatic_aberration_strength,
    normalize_chart_crosshair_scale, normalize_chart_dotted_background_opacity,
    normalize_chart_edge_blur_strength, normalize_chart_fisheye_strength,
    normalize_market_slippage_pct, normalize_pane_border_thickness, normalize_pane_corner_radius,
    normalize_ui_scale,
};
use crate::market_state::SymbolSearchMarketFilter;
use crate::message::Message;
use iced::Task;
#[cfg(target_os = "linux")]
use iced::window;

mod fonts;
mod hotkeys;
mod muted_tickers;
mod sounds;

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
            Message::ChartHollowCandleModeChanged(mode)
                if self.chart_hollow_candle_mode != mode =>
            {
                self.chart_hollow_candle_mode = mode;
                self.sync_chart_hollow_candles();
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
            | Message::TestChartHudOrderSound) => {
                return self.update_sound_preferences(message);
            }
            Message::ChartBackfillSourceChanged(source) if self.chart_backfill_source != source => {
                self.chart_backfill_source = source;
                self.journal.clear_snapshot_cache();
                self.journal.expanded_snapshot_trade_ids.clear();
                self.persist_config();
                self.push_toast(
                    format!("Chart backfill source set to {}", source.label()),
                    false,
                );
                return self.reload_chart_backfills_for_source_change();
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
                let scrub_task = self.scrub_hidden_symbol_state();
                self.refresh_symbol_search_results();
                self.refresh_live_watchlist_row_caches();
                self.persist_config();
                let account_task = self.refresh_account_data();
                return Task::batch([
                    stop_chase_task,
                    stop_twap_task,
                    scrub_task,
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
            | Message::KeyboardEvent(_, _)
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
        if let Some(id) = self.wallet_tracker.window_id {
            ids.push(id);
        }
        if let Some(id) = self.journal.window_id {
            ids.push(id);
        }

        ids.extend(self.wallet_detail_windows.keys().copied());
        ids.extend(self.twap_orders.values().filter_map(|twap| twap.window_id));
        ids.extend(self.advanced_order_history_windows.keys().copied());
        ids.extend(self.pnl_card_windows.keys().copied());
        ids.extend(self.detached_chart_windows.keys().copied());

        ids
    }
}

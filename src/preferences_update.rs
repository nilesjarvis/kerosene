use crate::app_state::TradingTerminal;
use crate::config::{
    normalize_market_slippage_pct, normalize_pane_border_thickness, normalize_pane_corner_radius,
    normalize_ui_scale,
};
use crate::market_state::SymbolSearchMarketFilter;
use crate::message::Message;
use iced::Task;

mod fonts;
mod hotkeys;
mod muted_tickers;

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
            message @ (Message::DisplayFontChanged(_)
            | Message::ImportDisplayFont
            | Message::DisplayFontImported(_)) => {
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
}

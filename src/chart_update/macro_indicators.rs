use crate::app_state::TradingTerminal;
use crate::chart_indicator::ChartIndicatorId;
use crate::message::Message;
use crate::timeframe::Timeframe;

use iced::Task;

impl TradingTerminal {
    /// Extend history only when an enabled average needs more source candles.
    fn ensure_moving_average_history(
        &mut self,
        id: crate::chart_state::ChartId,
        key: ChartIndicatorId,
    ) -> Task<Message> {
        let Some(inst) = self.charts.get(&id) else {
            return Task::none();
        };
        let Some(period) = key.period(&inst.macro_indicators) else {
            return Task::none();
        };
        if !key.is_enabled(inst)
            || inst.symbol.is_empty()
            || self.symbol_key_is_hidden(&inst.symbol)
        {
            return Task::none();
        }
        let candles = match key.group() {
            "chart_timeframe" => &inst.chart.candles,
            "hourly" => &inst.chart.hourly_candles,
            "daily" => &inst.chart.daily_candles,
            "weekly" => &inst.chart.weekly_candles,
            "monthly" => &inst.chart.monthly_candles,
            _ => return Task::none(),
        };
        if candles.len() >= period {
            return Task::none();
        }
        let symbol = inst.symbol.clone();
        let timeframe = inst.interval;
        if key.group() == "chart_timeframe" {
            self.queue_candle_fetch_for(id, &symbol, timeframe, None)
        } else {
            Task::batch(self.queue_macro_candles_tasks(id, &symbol))
        }
    }

    pub(super) fn update_chart_macro_indicators(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleMacroMenu(id) => {
                let opening = self
                    .charts
                    .get(&id)
                    .is_some_and(|inst| !inst.macro_menu_open);
                if opening {
                    self.close_chart_header_menus();
                }
                if let Some(inst) = self.charts.get_mut(&id) {
                    inst.macro_menu_open = opening;
                    if opening {
                        inst.moving_average_period_inputs.clear();
                    }
                }
            }
            Message::ChartMovingAveragePeriodChanged(id, key, value) => {
                let Some(default_period) = key.default_period() else {
                    return Task::none();
                };
                if value.len() > 4 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Task::none();
                }
                let period = value.parse::<usize>().ok();
                if period.is_some_and(|period| period > ChartIndicatorId::MAX_MOVING_AVERAGE_PERIOD)
                {
                    return Task::none();
                }
                let Some(inst) = self.charts.get_mut(&id) else {
                    return Task::none();
                };
                inst.moving_average_period_inputs.insert(key, value);
                let Some(period) = period.filter(|period| *period > 0) else {
                    return Task::none();
                };
                if key.period(&inst.macro_indicators) == Some(period) {
                    return Task::none();
                }
                if period == default_period {
                    inst.macro_indicators
                        .moving_average_periods
                        .remove(key.key());
                } else {
                    inst.macro_indicators
                        .moving_average_periods
                        .insert(key.key().to_string(), period);
                }
                inst.chart.macro_indicators = inst.macro_indicators.clone();
                inst.chart.candle_cache.clear();
                self.persist_config();
                return self.ensure_moving_average_history(id, key);
            }
            Message::ToggleMacroIndicator(id, key) => {
                let hydromancer_key_missing = self.hydromancer_api_key.trim().is_empty();
                let mut fetch_funding = false;
                let mut macro_symbol = None;
                let mut show_funding_key_prompt = false;
                if let Some(inst) = self.charts.get_mut(&id) {
                    let enabled = !key.is_enabled(inst);
                    key.set_enabled(inst, enabled);
                    if key == ChartIndicatorId::FundingRate {
                        if enabled {
                            fetch_funding = true;
                            show_funding_key_prompt = hydromancer_key_missing;
                        } else {
                            Self::clear_funding_display(inst);
                        }
                    }
                    if key.is_macro() {
                        if enabled {
                            macro_symbol = Some(inst.symbol.clone());
                        }
                        inst.chart.macro_indicators = inst.macro_indicators.clone();
                    }
                    inst.chart.candle_cache.clear();
                    self.persist_config();
                }
                if show_funding_key_prompt {
                    self.push_toast(
                        "Add a Hydromancer API key in Settings > Integrations to load Funding"
                            .to_string(),
                        true,
                    );
                }
                let mut tasks = macro_symbol
                    .map(|symbol| self.queue_macro_candles_tasks(id, &symbol))
                    .unwrap_or_default();
                if fetch_funding {
                    tasks.push(self.maybe_fetch_chart_funding(id));
                }
                if key.group() == "chart_timeframe" {
                    tasks.push(self.ensure_moving_average_history(id, key));
                }
                return Task::batch(tasks);
            }
            Message::MacroCandlesLoaded(id, request_id, symbol, tf, result) => {
                if self.symbol_key_is_hidden(&symbol) {
                    return Task::none();
                }
                if let Some(inst) = self.charts.get_mut(&id)
                    && inst.macro_candles_request_id == request_id
                    && inst.symbol == symbol
                    && let Ok(candles) = result
                {
                    match tf {
                        Timeframe::H1 => {
                            inst.chart.hourly_candles = candles;
                        }
                        Timeframe::D1 => {
                            inst.chart.daily_candles = candles;
                        }
                        Timeframe::W1 => {
                            inst.chart.weekly_candles = candles;
                        }
                        Timeframe::Mo1 => {
                            inst.chart.monthly_candles = candles;
                        }
                        _ => {}
                    }
                    inst.chart.candle_cache.clear();
                }
            }
            _ => {}
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Candle;
    use crate::chart_state::ChartInstance;
    use crate::timeframe::Timeframe;

    #[test]
    fn session_indicator_toggle_updates_canvas_state_and_snapshot() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();
        terminal
            .charts
            .insert(7, ChartInstance::new(7, "BTC".to_string(), Timeframe::H1));

        let _task = terminal.update_chart_macro_indicators(Message::ToggleMacroIndicator(
            7,
            ChartIndicatorId::Sessions,
        ));

        let instance = terminal.charts.get(&7).expect("chart instance");
        assert!(instance.macro_indicators.show_session_indicator);
        assert!(instance.chart.macro_indicators.show_session_indicator);
        assert!(
            terminal
                .chart_configs_snapshot()
                .iter()
                .any(|config| config.id == 7 && config.macro_indicators.show_session_indicator)
        );
    }

    #[test]
    fn high_low_toggle_updates_canvas_state_and_snapshot() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();
        terminal
            .charts
            .insert(7, ChartInstance::new(7, "BTC".to_string(), Timeframe::H1));

        let _task = terminal.update_chart_macro_indicators(Message::ToggleMacroIndicator(
            7,
            ChartIndicatorId::HighLow,
        ));

        let instance = terminal.charts.get(&7).expect("chart instance");
        assert!(instance.macro_indicators.show_high_low);
        assert!(instance.chart.macro_indicators.show_high_low);
        assert!(
            terminal
                .chart_configs_snapshot()
                .iter()
                .any(|config| config.id == 7 && config.macro_indicators.show_high_low)
        );
    }

    #[test]
    fn leledc_toggles_update_canvas_state_and_snapshot() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();
        terminal
            .charts
            .insert(7, ChartInstance::new(7, "BTC".to_string(), Timeframe::H1));

        let _task = terminal.update_chart_macro_indicators(Message::ToggleMacroIndicator(
            7,
            ChartIndicatorId::LeledcArrows,
        ));
        let _task = terminal.update_chart_macro_indicators(Message::ToggleMacroIndicator(
            7,
            ChartIndicatorId::LeledcLevels,
        ));

        let instance = terminal.charts.get(&7).expect("chart instance");
        assert!(instance.macro_indicators.show_leledc_arrows);
        assert!(instance.macro_indicators.show_leledc_levels);
        assert!(instance.chart.macro_indicators.show_leledc_arrows);
        assert!(instance.chart.macro_indicators.show_leledc_levels);
        assert!(terminal.chart_configs_snapshot().iter().any(|config| {
            config.id == 7
                && config.macro_indicators.show_leledc_arrows
                && config.macro_indicators.show_leledc_levels
        }));
    }

    #[test]
    fn stale_macro_candle_result_does_not_overwrite_current_batch() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();

        let mut instance = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
        instance.macro_candles_request_id = 2;
        instance.chart.daily_candles = vec![Candle::test_flat(2_000, 200.0)];
        terminal.charts.insert(7, instance);

        let _task = terminal.update_chart_macro_indicators(Message::MacroCandlesLoaded(
            7,
            1,
            "BTC".to_string(),
            Timeframe::D1,
            Ok(vec![Candle::test_flat(1_000, 100.0)]),
        ));

        let instance = terminal.charts.get(&7).expect("chart instance");
        assert_eq!(instance.macro_candles_request_id, 2);
        assert_eq!(instance.chart.daily_candles.len(), 1);
        assert_eq!(instance.chart.daily_candles[0].open_time, 2_000);
        assert_eq!(instance.chart.daily_candles[0].close, 200.0);
    }

    #[test]
    fn current_macro_candle_result_updates_matching_batch() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();

        let mut instance = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
        instance.macro_candles_request_id = 2;
        terminal.charts.insert(7, instance);

        let _task = terminal.update_chart_macro_indicators(Message::MacroCandlesLoaded(
            7,
            2,
            "BTC".to_string(),
            Timeframe::W1,
            Ok(vec![Candle::test_flat(3_000, 300.0)]),
        ));

        let instance = terminal.charts.get(&7).expect("chart instance");
        assert_eq!(instance.chart.weekly_candles.len(), 1);
        assert_eq!(instance.chart.weekly_candles[0].open_time, 3_000);
        assert_eq!(instance.chart.weekly_candles[0].close, 300.0);
    }

    #[test]
    fn current_hourly_macro_candle_result_updates_hourly_batch() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();

        let mut instance = ChartInstance::new(7, "BTC".to_string(), Timeframe::H4);
        instance.macro_candles_request_id = 2;
        terminal.charts.insert(7, instance);

        let _task = terminal.update_chart_macro_indicators(Message::MacroCandlesLoaded(
            7,
            2,
            "BTC".to_string(),
            Timeframe::H1,
            Ok(vec![Candle::test_flat(4_000, 400.0)]),
        ));

        let instance = terminal.charts.get(&7).expect("chart instance");
        assert_eq!(instance.chart.hourly_candles.len(), 1);
        assert_eq!(instance.chart.hourly_candles[0].open_time, 4_000);
        assert_eq!(instance.chart.hourly_candles[0].close, 400.0);
    }
    #[test]
    fn moving_average_period_changes_update_only_target_chart_and_survive_restoration() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();
        for id in [7, 8] {
            terminal
                .charts
                .insert(id, ChartInstance::new(id, "BTC".into(), Timeframe::H1));
        }
        let key = ChartIndicatorId::TfEma50;
        let _task = terminal.update_chart_macro_indicators(
            Message::ChartMovingAveragePeriodChanged(7, key, "21".into()),
        );
        let instance = &terminal.charts[&7];
        assert_eq!(key.period(&instance.macro_indicators), Some(21));
        assert_eq!(key.period(&instance.chart.macro_indicators), Some(21));
        assert_eq!(key.period(&terminal.charts[&8].macro_indicators), Some(50));
        assert!(!key.is_enabled(instance));
        let detached = instance.clone_for_detached_window(9);
        assert_eq!(key.period(&detached.chart.macro_indicators), Some(21));
        assert!(detached.moving_average_period_inputs.is_empty());
        let configs = terminal.chart_configs_snapshot();
        let config = configs
            .iter()
            .find(|config| config.id == 7)
            .expect("chart snapshot");
        let restored = ChartInstance::from_config(config, config.symbol.clone());
        assert_eq!(key.period(&restored.chart.macro_indicators), Some(21));
        assert!(terminal.config_save_due_at.is_some());
    }

    #[test]
    fn moving_average_invalid_edits_preserve_last_valid_period() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();
        terminal
            .charts
            .insert(7, ChartInstance::new(7, "BTC".into(), Timeframe::H1));
        let key = ChartIndicatorId::TfSma50;
        let _task = terminal.update_chart_macro_indicators(
            Message::ChartMovingAveragePeriodChanged(7, key, "9".into()),
        );
        for value in ["", "0", "-1", "1.5", "abc", "5001", "99999999999999999999"] {
            let _task = terminal.update_chart_macro_indicators(
                Message::ChartMovingAveragePeriodChanged(7, key, value.into()),
            );
            assert_eq!(
                key.period(&terminal.charts[&7].chart.macro_indicators),
                Some(9)
            );
        }
        let _task = terminal.update_chart_macro_indicators(
            Message::ChartMovingAveragePeriodChanged(7, ChartIndicatorId::FundingRate, "21".into()),
        );
        assert_eq!(
            terminal.charts[&7]
                .macro_indicators
                .moving_average_periods
                .len(),
            1
        );
        let _task = terminal.update_chart_macro_indicators(Message::ToggleMacroMenu(7));
        assert!(terminal.charts[&7].moving_average_period_inputs.is_empty());
        let _task = terminal.update_chart_macro_indicators(
            Message::ChartMovingAveragePeriodChanged(7, key, "50".into()),
        );
        assert!(
            terminal.charts[&7]
                .macro_indicators
                .moving_average_periods
                .is_empty()
        );
    }

    #[test]
    fn moving_average_larger_period_requests_more_current_timeframe_history() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();
        let mut instance = ChartInstance::new(7, "BTC".into(), Timeframe::H1);
        instance.macro_indicators.tf_ema_50 = true;
        terminal.charts.insert(7, instance);
        let _task = terminal.update_chart_macro_indicators(
            Message::ChartMovingAveragePeriodChanged(7, ChartIndicatorId::TfEma50, "1000".into()),
        );
        let request = terminal.charts[&7]
            .candle_fetch_request
            .as_ref()
            .expect("history request");
        assert_eq!(
            request.end_ms - request.start_ms,
            Timeframe::H1.lookback_ms() + 3_000 * Timeframe::H1.duration_ms()
        );
    }
}

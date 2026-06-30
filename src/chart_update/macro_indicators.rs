use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::timeframe::Timeframe;

use iced::Task;

impl TradingTerminal {
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
                }
            }
            Message::ToggleMacroIndicator(id, key) => {
                let hydromancer_key_missing = self.hydromancer_api_key.trim().is_empty();
                let mut fetch_funding = false;
                let mut show_funding_key_prompt = false;
                if let Some(inst) = self.charts.get_mut(&id) {
                    match key.as_str() {
                        "tf_sma_50" => {
                            inst.macro_indicators.tf_sma_50 = !inst.macro_indicators.tf_sma_50
                        }
                        "tf_ema_50" => {
                            inst.macro_indicators.tf_ema_50 = !inst.macro_indicators.tf_ema_50
                        }
                        "tf_sma_200" => {
                            inst.macro_indicators.tf_sma_200 = !inst.macro_indicators.tf_sma_200
                        }
                        "tf_ema_200" => {
                            inst.macro_indicators.tf_ema_200 = !inst.macro_indicators.tf_ema_200
                        }
                        "sma_50h" => inst.macro_indicators.sma_50h = !inst.macro_indicators.sma_50h,
                        "ema_50h" => inst.macro_indicators.ema_50h = !inst.macro_indicators.ema_50h,
                        "sma_200h" => {
                            inst.macro_indicators.sma_200h = !inst.macro_indicators.sma_200h
                        }
                        "ema_200h" => {
                            inst.macro_indicators.ema_200h = !inst.macro_indicators.ema_200h
                        }
                        "sma_50d" => inst.macro_indicators.sma_50d = !inst.macro_indicators.sma_50d,
                        "ema_50d" => inst.macro_indicators.ema_50d = !inst.macro_indicators.ema_50d,
                        "sma_200d" => {
                            inst.macro_indicators.sma_200d = !inst.macro_indicators.sma_200d
                        }
                        "ema_200d" => {
                            inst.macro_indicators.ema_200d = !inst.macro_indicators.ema_200d
                        }
                        "sma_20w" => inst.macro_indicators.sma_20w = !inst.macro_indicators.sma_20w,
                        "ema_20w" => inst.macro_indicators.ema_20w = !inst.macro_indicators.ema_20w,
                        "sma_50w" => inst.macro_indicators.sma_50w = !inst.macro_indicators.sma_50w,
                        "ema_50w" => inst.macro_indicators.ema_50w = !inst.macro_indicators.ema_50w,
                        "sma_12m" => inst.macro_indicators.sma_12m = !inst.macro_indicators.sma_12m,
                        "ema_12m" => inst.macro_indicators.ema_12m = !inst.macro_indicators.ema_12m,
                        "show_funding_rate" => {
                            inst.macro_indicators.show_funding_rate =
                                !inst.macro_indicators.show_funding_rate;
                            if inst.macro_indicators.show_funding_rate {
                                fetch_funding = true;
                                show_funding_key_prompt = hydromancer_key_missing;
                            } else {
                                Self::clear_funding_display(inst);
                            }
                        }
                        "show_session_indicator" => {
                            inst.macro_indicators.show_session_indicator =
                                !inst.macro_indicators.show_session_indicator
                        }
                        "show_labels" => {
                            inst.macro_indicators.show_labels = !inst.macro_indicators.show_labels
                        }
                        "show_volume_profile" => {
                            inst.macro_indicators.show_volume_profile =
                                !inst.macro_indicators.show_volume_profile
                        }
                        _ => {}
                    }
                    inst.chart.macro_indicators = inst.macro_indicators.clone();
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
                if fetch_funding {
                    return self.maybe_fetch_chart_funding(id);
                }
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
            "show_session_indicator".to_string(),
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
}

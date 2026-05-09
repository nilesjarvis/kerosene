use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::timeframe::Timeframe;

use iced::Task;

impl TradingTerminal {
    pub(super) fn update_chart_macro_indicators(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleMacroMenu(id) => {
                if let Some(inst) = self.charts.get_mut(&id) {
                    inst.macro_menu_open = !inst.macro_menu_open;
                }
            }
            Message::ToggleMacroIndicator(id, key) => {
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
                        "show_labels" => {
                            inst.macro_indicators.show_labels = !inst.macro_indicators.show_labels
                        }
                        _ => {}
                    }
                    inst.chart.macro_indicators = inst.macro_indicators.clone();
                    inst.chart.candle_cache.clear();
                    self.persist_config();
                }
            }
            Message::MacroCandlesLoaded(id, symbol, tf, result) => {
                if self.is_ticker_muted(&symbol) {
                    return Task::none();
                }
                if let Some(inst) = self.charts.get_mut(&id)
                    && inst.symbol == symbol
                    && let Ok(candles) = result
                {
                    match tf {
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

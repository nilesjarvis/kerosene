use crate::app_state::TradingTerminal;
use crate::config::normalize_market_slippage_pct;
use crate::message::Message;
use iced::Task;

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
            Message::MutedTickerInputChanged(value) => {
                self.muted_ticker_input = value;
                self.muted_ticker_status = None;
            }
            message @ (Message::MuteTicker | Message::UnmuteTicker(_)) => {
                return self.update_muted_ticker_preferences(message);
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
            | Message::KeyboardEvent(_, _)
            | Message::ExecuteHotkey(_)) => return self.update_hotkey_preferences(message),
            _ => {}
        }

        Task::none()
    }
}

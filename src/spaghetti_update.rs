mod creation;
mod data;
mod editor;
mod pair;
mod session;
mod style;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Task;

impl TradingTerminal {
    pub(crate) fn update_spaghetti(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddComparisonChart => self.add_comparison_chart(),
            Message::AddPairRatioChart => self.add_pair_ratio_chart(),
            Message::SpaghettiReload(id) => self.reload_spaghetti_chart(id),
            Message::SpaghettiSwitchTimeframe(id, tf) => self.switch_spaghetti_timeframe(id, tf),
            Message::SpaghettiCandlesLoaded(request, result) => {
                self.apply_spaghetti_candles_loaded(request, result)
            }
            Message::SpaghettiWsCandleUpdate(id, symbol, candle) => {
                self.apply_spaghetti_ws_candle_update(id, symbol, candle)
            }
            Message::SpaghettiOpenEditor(id) => self.open_spaghetti_editor(id),
            Message::SpaghettiCloseEditor(id) => self.close_spaghetti_editor(id),
            Message::SpaghettiEditorSearchChanged(id, query) => {
                self.update_spaghetti_editor_search(id, query)
            }
            Message::SpaghettiAddSymbol(id, key) => self.add_spaghetti_symbol(id, key),
            Message::SpaghettiRemoveSymbol(id, symbol) => self.remove_spaghetti_symbol(id, symbol),
            Message::SpaghettiSetSession(id, session) => self.set_spaghetti_session(id, session),
            Message::SpaghettiSetSessionGranularityAuto(id) => {
                self.set_spaghetti_session_granularity_auto(id)
            }
            Message::SpaghettiResetView(id) => self.reset_spaghetti_view(id),
            Message::ToggleSpaghettiStyleMenu(id) => self.toggle_spaghetti_style_menu(id),
            Message::ToggleSpaghettiLabels(id) => self.toggle_spaghetti_labels(id),
            Message::SpaghettiSetColorMode(id, mode) => self.set_spaghetti_color_mode(id, mode),
            Message::PairSetCandleMode(id, enabled) => self.set_pair_candle_mode(id, enabled),
            _ => Task::none(),
        }
    }
}

mod creation;
mod data;
mod editor;
mod pair;
mod session;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Task;

impl TradingTerminal {
    pub(crate) fn update_spaghetti(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddComparisonChart => self.add_comparison_chart(),
            Message::AddPairTradeChart => self.add_pair_trade_chart(),
            Message::SpaghettiReload(id) => self.reload_spaghetti_chart(id),
            Message::SpaghettiSwitchTimeframe(id, tf) => self.switch_spaghetti_timeframe(id, tf),
            Message::SpaghettiCandlesLoaded(id, symbol, result) => {
                self.apply_spaghetti_candles_loaded(id, symbol, result)
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
            Message::PairNotionalChanged(id, value) => self.update_pair_notional(id, value),
            Message::PairSetCandleMode(id, enabled) => self.set_pair_candle_mode(id, enabled),
            Message::PairExecute(id, long_a_short_b) => self.execute_pair_trade(id, long_a_short_b),
            Message::PairExecutionDone(id, result) => self.finish_pair_execution(id, *result),
            _ => Task::none(),
        }
    }
}

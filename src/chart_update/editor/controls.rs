use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_chart_editor_controls(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleChartInvert(id) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.chart.inverted = !instance.chart.inverted;
                    instance.chart.candle_cache.clear();
                    self.persist_config();
                }
            }
            Message::ToggleChartTradeMarkers(id) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.chart.show_trade_markers = !instance.chart.show_trade_markers;
                    self.persist_config();
                }
            }
            Message::ChartOpenEditor(id) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.editor_open = true;
                    instance.editor_search_query.clear();
                    instance.editor_keyboard_selected = false;
                }
                return iced::widget::operation::focus(Self::chart_symbol_search_input_id(id));
            }
            Message::ChartCloseEditor(id) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.editor_open = false;
                    instance.editor_search_query.clear();
                    instance.editor_keyboard_selected = false;
                }
            }
            Message::ChartEditorSearchChanged(id, query) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.editor_search_query = query;
                    instance.editor_keyboard_selected = false;
                }
            }
            Message::ChartEditorSubmit(id) => {
                let query = self
                    .charts
                    .get(&id)
                    .map(|instance| instance.editor_search_query.trim().to_string())
                    .unwrap_or_default();

                if query.is_empty() {
                    return Task::none();
                }

                if let Some(key) = self
                    .chart_editor_filtered_symbols(&query)
                    .first()
                    .map(|symbol| symbol.key.clone())
                {
                    return self.update(Message::ChartSymbolSelected(id, key));
                }

                self.push_toast(format!("No symbol matches '{query}'"), true);
            }
            _ => {}
        }

        Task::none()
    }
}

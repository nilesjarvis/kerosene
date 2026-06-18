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
                    instance.editor_selected_index = None;
                    instance.secondary_editor_open = false;
                    instance.secondary_editor_search_query.clear();
                    instance.secondary_editor_selected_index = None;
                }
                return Task::batch([
                    Self::focus_chart_symbol_search_input(id),
                    Self::scroll_chart_symbol_search_results_to(id, 0.0),
                ]);
            }
            Message::ChartCloseEditor(id) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.editor_open = false;
                    instance.editor_search_query.clear();
                    instance.editor_selected_index = None;
                }
            }
            Message::ChartEditorSearchChanged(id, query) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.editor_search_query = query;
                    instance.editor_selected_index = None;
                }
                return Self::scroll_chart_symbol_search_results_to(id, 0.0);
            }
            Message::ChartEditorSubmit(id) => {
                let (query, selected_index) = self
                    .charts
                    .get(&id)
                    .map(|instance| {
                        (
                            instance.editor_search_query.trim().to_string(),
                            instance.editor_selected_index,
                        )
                    })
                    .unwrap_or_default();

                let filtered = self.chart_editor_filtered_symbols(&query);
                let selected_symbol = selected_index
                    .and_then(|index| filtered.get(index))
                    .or_else(|| (!query.is_empty()).then(|| filtered.first()).flatten());

                if let Some(key) = selected_symbol.map(|symbol| symbol.key.clone()) {
                    return self.update(Message::ChartSymbolSelected(id, key));
                }

                if !query.is_empty() {
                    self.push_toast(format!("No symbol matches '{query}'"), true);
                }
            }
            Message::ChartSecondaryOpenEditor(id) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.editor_open = false;
                    instance.editor_search_query.clear();
                    instance.editor_selected_index = None;
                    instance.secondary_editor_open = true;
                    instance.secondary_editor_search_query.clear();
                    instance.secondary_editor_selected_index = None;
                }
                return Task::batch([
                    Self::focus_chart_secondary_symbol_search_input(id),
                    Self::scroll_chart_secondary_symbol_search_results_to(id, 0.0),
                ]);
            }
            Message::ChartSecondaryCloseEditor(id) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.secondary_editor_open = false;
                    instance.secondary_editor_search_query.clear();
                    instance.secondary_editor_selected_index = None;
                }
            }
            Message::ChartSecondaryEditorSearchChanged(id, query) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.secondary_editor_search_query = query;
                    instance.secondary_editor_selected_index = None;
                }
                return Self::scroll_chart_secondary_symbol_search_results_to(id, 0.0);
            }
            Message::ChartSecondaryEditorSubmit(id) => {
                let (query, selected_index) = self
                    .charts
                    .get(&id)
                    .map(|instance| {
                        (
                            instance.secondary_editor_search_query.trim().to_string(),
                            instance.secondary_editor_selected_index,
                        )
                    })
                    .unwrap_or_default();

                let filtered = self.chart_editor_filtered_symbols(&query);
                let selected_symbol = selected_index
                    .and_then(|index| filtered.get(index))
                    .or_else(|| (!query.is_empty()).then(|| filtered.first()).flatten());

                if let Some(key) = selected_symbol.map(|symbol| symbol.key.clone()) {
                    return self.update(Message::ChartSecondarySymbolSelected(id, key));
                }

                if !query.is_empty() {
                    self.push_toast(format!("No symbol matches '{query}'"), true);
                }
            }
            Message::ChartSecondarySymbolRemoved(id) => {
                if let Some(instance) = self.charts.get_mut(&id) {
                    instance.clear_secondary_symbol();
                    self.persist_config();
                }
            }
            _ => {}
        }

        Task::none()
    }
}

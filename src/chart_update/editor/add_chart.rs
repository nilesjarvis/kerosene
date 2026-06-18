use crate::app_state::TradingTerminal;
use crate::chart_state::ChartInstance;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;

impl TradingTerminal {
    pub(super) fn add_chart_pane(&mut self, message: Message) -> Task<Message> {
        let Message::AddChart(from_pane) = message else {
            return Task::none();
        };

        self.add_widget_menu_open = false;
        let id = self.alloc_chart_id();
        let mut instance = ChartInstance::new_empty(id);
        self.apply_chart_appearance_settings(&mut instance.chart);
        self.charts.insert(id, instance);
        if self
            .add_pane_to_target(
                self.add_widget_axis(),
                from_pane,
                PaneKind::Chart(id),
                "Candlestick Chart",
            )
            .is_some()
        {
            self.primary_chart_id = Some(id);
            return iced::widget::operation::focus(Self::chart_symbol_search_input_id(id));
        }
        self.charts.remove(&id);

        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use crate::app_state::TradingTerminal;
    use crate::chart::CandlestickChart;
    use crate::config::{ChartHollowCandleMode, ChartSeriesStyle, KeroseneConfig};

    #[test]
    fn new_charts_inherit_global_appearance_settings() {
        let (terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig {
            chart_series_style: ChartSeriesStyle::Line,
            chart_hollow_candle_mode: ChartHollowCandleMode::Both,
            ..KeroseneConfig::default()
        });

        let mut chart = CandlestickChart::new(999);
        terminal.apply_chart_appearance_settings(&mut chart);

        assert_eq!(chart.series_style, ChartSeriesStyle::Line);
        assert_eq!(chart.hollow_candle_mode, ChartHollowCandleMode::Both);
    }
}

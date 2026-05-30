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
        let (bull, bear) = self.active_chart_theme_colors();
        instance.chart.set_chart_colors(bull, bear);
        instance.chart.set_dotted_background(
            self.chart_dotted_background,
            self.chart_dotted_background_opacity,
        );
        instance
            .chart
            .set_hollow_candle_mode(self.chart_hollow_candle_mode);
        instance
            .chart
            .set_fisheye(self.chart_fisheye_enabled, self.chart_fisheye_strength);
        instance.chart.set_chromatic_aberration(
            self.chart_chromatic_aberration_enabled,
            self.chart_chromatic_aberration_strength,
        );
        instance
            .chart
            .set_edge_blur(self.chart_edge_blur_enabled, self.chart_edge_blur_strength);
        instance
            .chart
            .set_crosshair_style(self.chart_crosshair_style);
        instance
            .chart
            .set_crosshair_guides_enabled(self.chart_crosshair_guides_enabled);
        instance
            .chart
            .set_crosshair_scale(self.chart_crosshair_scale);
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

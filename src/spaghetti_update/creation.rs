use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::spaghetti_state::SpaghettiChartInstance;

use iced::Task;

impl TradingTerminal {
    pub(super) fn add_comparison_chart(&mut self) -> Task<Message> {
        self.add_spaghetti_chart(SpaghettiChartInstance::new_empty, "Comparison Chart")
    }

    pub(super) fn add_pair_ratio_chart(&mut self) -> Task<Message> {
        self.add_spaghetti_chart(SpaghettiChartInstance::new_pair, "Pair Ratio")
    }

    fn add_spaghetti_chart(
        &mut self,
        build_instance: impl FnOnce(u64) -> SpaghettiChartInstance,
        title: &'static str,
    ) -> Task<Message> {
        self.add_widget_menu_open = false;
        let id = self.next_spaghetti_id;
        self.next_spaghetti_id += 1;
        let mut instance = build_instance(id);
        instance.canvas.set_dotted_background(
            self.chart_dotted_background,
            self.chart_dotted_background_opacity,
        );
        instance
            .canvas
            .set_gradient_background(self.chart_gradient_background, self.chart_gradient_contrast);
        instance
            .canvas
            .set_hollow_candle_mode(self.chart_hollow_candle_mode);
        instance
            .canvas
            .set_crosshair_style(self.chart_crosshair_style);
        instance
            .canvas
            .set_crosshair_guides_enabled(self.chart_crosshair_guides_enabled);
        instance
            .canvas
            .set_crosshair_scale(self.chart_crosshair_scale);
        self.spaghetti_charts.insert(id, instance);
        if self
            .add_pane_next_to_focus(self.add_widget_axis(), PaneKind::SpaghettiChart(id), title)
            .is_none()
        {
            self.spaghetti_charts.remove(&id);
        }
        Task::none()
    }
}

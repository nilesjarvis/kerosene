use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(crate) fn update_annotations(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetDrawingTool(chart_id, surface_id, tool) => {
                if !self.charts.contains_key(&chart_id) {
                    return Task::none();
                }
                if let Some(tool) = tool {
                    self.chart_surface_active_tools.insert(surface_id, tool);
                } else {
                    self.chart_surface_active_tools.remove(&surface_id);
                }
                if let Some(instance) = self.charts.get_mut(&chart_id)
                    && instance.chart.surface_id() == surface_id
                {
                    instance.chart.active_tool = tool;
                }
            }
            Message::AddAnnotation(chart_id, mut annotation) => {
                if !annotation.is_valid() {
                    self.order_status = Some(("Invalid chart annotation".into(), true));
                    return Task::none();
                }
                if let Some(instance) = self.charts.get_mut(&chart_id) {
                    annotation.id = instance.next_annotation_id;
                    instance.next_annotation_id += 1;
                    instance.annotations.push(annotation.clone());
                    instance.chart.annotations = instance.annotations.clone();
                }
                self.persist_config();
            }
            Message::RemoveAnnotation(chart_id, annotation_id) => {
                if let Some(instance) = self.charts.get_mut(&chart_id) {
                    instance.annotations.retain(|a| a.id != annotation_id);
                    instance.chart.annotations = instance.annotations.clone();
                }
                self.persist_config();
            }
            Message::ClearDrawingTool(chart_id, surface_id) => {
                self.chart_surface_active_tools.remove(&surface_id);
                if let Some(instance) = self.charts.get_mut(&chart_id)
                    && instance.chart.surface_id() == surface_id
                {
                    instance.chart.active_tool = None;
                }
            }
            Message::EscapePressed(window_id)
                if self
                    .main_window_id
                    .is_none_or(|main_id| main_id == window_id) =>
            {
                self.chart_surface_active_tools.clear();
                for instance in self.charts.values_mut() {
                    instance.chart.active_tool = None;
                }
            }
            _ => {}
        }

        Task::none()
    }
}

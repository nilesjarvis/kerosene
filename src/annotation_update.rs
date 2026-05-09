use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(crate) fn update_annotations(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetDrawingTool(chart_id, tool) => {
                if let Some(instance) = self.charts.get_mut(&chart_id) {
                    instance.chart.active_tool = tool;
                }
            }
            Message::AddAnnotation(mut annotation) => {
                if !annotation.is_valid() {
                    self.order_status = Some(("Invalid chart annotation".into(), true));
                    return Task::none();
                }
                if let Some(id) = self.primary_chart_id
                    && let Some(instance) = self.charts.get_mut(&id)
                {
                    annotation.id = instance.next_annotation_id;
                    instance.next_annotation_id += 1;
                    instance.annotations.push(annotation.clone());
                    instance.chart.annotations = instance.annotations.clone();
                }
                self.persist_config();
            }
            Message::RemoveAnnotation(annotation_id) => {
                if let Some(id) = self.primary_chart_id
                    && let Some(instance) = self.charts.get_mut(&id)
                {
                    instance.annotations.retain(|a| a.id != annotation_id);
                    instance.chart.annotations = instance.annotations.clone();
                }
                self.persist_config();
            }
            Message::ClearDrawingTool => {
                for instance in self.charts.values_mut() {
                    instance.chart.active_tool = None;
                }
            }
            _ => {}
        }

        Task::none()
    }
}

use crate::app_state::TradingTerminal;
use crate::chart_state::ChartInstance;
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
                    instance.annotations.push(annotation);
                    mirror_annotations(instance);
                }
                self.persist_config();
            }
            Message::RemoveAnnotation(chart_id, annotation_id) => {
                if let Some(instance) = self.charts.get_mut(&chart_id) {
                    instance.annotations.retain(|a| a.id != annotation_id);
                    if instance.selected_annotation == Some(annotation_id) {
                        instance.selected_annotation = None;
                    }
                    mirror_annotations(instance);
                }
                self.persist_config();
            }
            Message::UpdateAnnotation(chart_id, annotation) => {
                if !annotation.is_valid() {
                    return Task::none();
                }
                if let Some(instance) = self.charts.get_mut(&chart_id)
                    && let Some(slot) = instance
                        .annotations
                        .iter_mut()
                        .find(|a| a.id == annotation.id)
                {
                    *slot = annotation;
                    mirror_annotations(instance);
                    self.persist_config();
                }
            }
            Message::SelectAnnotation(chart_id, annotation_id) => {
                if let Some(instance) = self.charts.get_mut(&chart_id) {
                    instance.selected_annotation = annotation_id;
                }
            }
            Message::RestyleAnnotation(chart_id, annotation_id, style) => {
                if let Some(instance) = self.charts.get_mut(&chart_id)
                    && let Some(slot) = instance
                        .annotations
                        .iter_mut()
                        .find(|a| a.id == annotation_id)
                {
                    slot.style = style;
                    if slot.is_valid() {
                        mirror_annotations(instance);
                        self.persist_config();
                    }
                }
            }
            Message::ClearDrawingTool(chart_id, surface_id) => {
                self.chart_surface_active_tools.remove(&surface_id);
                if let Some(instance) = self.charts.get_mut(&chart_id)
                    && instance.chart.surface_id() == surface_id
                {
                    instance.chart.active_tool = None;
                }
            }
            _ => {}
        }

        Task::none()
    }
}

/// Keep the canvas-side annotation copy in sync with the persisted instance Vec.
fn mirror_annotations(instance: &mut ChartInstance) {
    instance.chart.annotations = instance.annotations.clone();
}

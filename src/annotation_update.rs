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
                    let Some(annotation) =
                        instance.annotations.iter().find(|a| a.id == annotation_id)
                    else {
                        return Task::none();
                    };
                    if annotation.style.locked {
                        return Task::none();
                    }
                    instance.annotations.retain(|a| a.id != annotation_id);
                    if instance.selected_annotation == Some(annotation_id) {
                        instance.selected_annotation = None;
                    }
                    mirror_annotations(instance);
                    self.persist_config();
                }
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
                    let mut candidate = slot.clone();
                    candidate.style = style;
                    if candidate.is_valid() {
                        *slot = candidate;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotations::{Annotation, AnnotationKind, AnnotationStyle};
    use crate::timeframe::Timeframe;

    fn level(id: u64) -> Annotation {
        Annotation {
            id,
            kind: AnnotationKind::HorizontalLevel { price: 100.0 },
            style: AnnotationStyle::default(),
        }
    }

    fn terminal_with_annotation(annotation: Annotation) -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();
        let mut instance = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
        instance.next_annotation_id = annotation.id + 1;
        instance.selected_annotation = Some(annotation.id);
        instance.annotations = vec![annotation];
        mirror_annotations(&mut instance);
        terminal.charts.insert(7, instance);
        terminal
    }

    #[test]
    fn remove_annotation_ignores_locked_annotation_messages() {
        let mut annotation = level(5);
        annotation.style.locked = true;
        let mut terminal = terminal_with_annotation(annotation);

        let _ = terminal.update_annotations(Message::RemoveAnnotation(7, 5));

        let instance = terminal.charts.get(&7).expect("chart exists");
        assert_eq!(instance.annotations.len(), 1);
        assert_eq!(instance.chart.annotations.len(), 1);
        assert_eq!(instance.selected_annotation, Some(5));
        assert!(instance.annotations[0].style.locked);
    }

    #[test]
    fn invalid_restyle_is_ignored_without_poisoning_chart_state() {
        let annotation = level(5);
        let original_style = annotation.style.clone();
        let mut terminal = terminal_with_annotation(annotation);
        let mut invalid_style = original_style.clone();
        invalid_style.width = f32::NAN;

        let _ = terminal.update_annotations(Message::RestyleAnnotation(7, 5, invalid_style));

        let instance = terminal.charts.get(&7).expect("chart exists");
        assert_eq!(instance.annotations[0].style, original_style);
        assert_eq!(instance.chart.annotations[0].style, original_style);
        assert!(instance.annotations[0].is_valid());
    }
}

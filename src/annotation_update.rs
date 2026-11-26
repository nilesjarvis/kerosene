use crate::annotations::{DrawingMenu, DrawingTool};
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartInstance;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(crate) fn update_annotations(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleDrawingMenu(chart_id, surface_id, menu) => {
                let Some(instance) = self.charts.get(&chart_id) else {
                    return Task::none();
                };
                if let DrawingMenu::Colors(id) = menu
                    && (instance.selected_annotation != Some(id)
                        || self.active_chart_surface_tool(chart_id, surface_id)
                            != Some(DrawingTool::Select))
                {
                    return Task::none();
                }
                let opening = instance.drawing_menu != Some((surface_id, menu));
                self.close_chart_header_menus();
                if opening && let Some(instance) = self.charts.get_mut(&chart_id) {
                    instance.drawing_menu = Some((surface_id, menu));
                }
            }
            Message::SetDrawingTool(chart_id, surface_id, tool) => {
                let Some(instance) = self.charts.get_mut(&chart_id) else {
                    return Task::none();
                };
                instance.drawing_menu = None;
                if let Some(tool) = tool {
                    self.chart_surface_active_tools.insert(surface_id, tool);
                } else {
                    self.chart_surface_active_tools.remove(&surface_id);
                }
                if instance.chart.surface_id() == surface_id {
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
                        instance.drawing_menu = None;
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
                    instance.drawing_menu = None;
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
                        if matches!(instance.drawing_menu, Some((_, DrawingMenu::Colors(id))) if id == annotation_id)
                        {
                            instance.drawing_menu = None;
                        }
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
                    instance.drawing_menu = None;
                    instance.selected_annotation = None;
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
    use crate::chart_state::ChartSurfaceId;
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
    fn choosing_a_tool_closes_the_palette_and_arms_only_its_surface() {
        let mut terminal = terminal_with_annotation(level(5));
        let surface = ChartSurfaceId::Docked(7);
        let detached = ChartSurfaceId::Detached(iced::window::Id::unique());
        let original_config = terminal.chart_configs_snapshot();
        let toggle = Message::ToggleDrawingMenu(7, surface, DrawingMenu::Tools);
        let _ = terminal.update(toggle.clone());
        assert_eq!(
            terminal.charts[&7].drawing_menu,
            Some((surface, DrawingMenu::Tools))
        );
        assert_eq!(terminal.chart_configs_snapshot(), original_config);
        let _ = terminal.update(Message::SetDrawingTool(7, surface, Some(DrawingTool::Ray)));
        assert_eq!(terminal.charts[&7].drawing_menu, None);
        assert_eq!(
            terminal.active_chart_surface_tool(7, surface),
            Some(DrawingTool::Ray)
        );
        assert_eq!(terminal.active_chart_surface_tool(7, detached), None);

        let _ = terminal.update(toggle);
        let _ = terminal.update(Message::CloseAllMenus);
        assert_eq!(terminal.charts[&7].drawing_menu, None);
        assert_eq!(
            terminal.active_chart_surface_tool(7, surface),
            Some(DrawingTool::Ray)
        );
    }

    #[test]
    fn drawing_colors_follow_the_selection_and_close_on_restyle_or_deselection() {
        let mut terminal = terminal_with_annotation(level(5));
        let surface = ChartSurfaceId::Docked(7);
        let colors = DrawingMenu::Colors(5);
        let toggle = Message::ToggleDrawingMenu(7, surface, colors);
        let _ = terminal.update(toggle.clone());
        assert_eq!(terminal.charts[&7].drawing_menu, None);
        let _ = terminal.update(Message::SetDrawingTool(
            7,
            surface,
            Some(DrawingTool::Select),
        ));
        let _ = terminal.update(toggle.clone());
        assert_eq!(terminal.charts[&7].drawing_menu, Some((surface, colors)));
        let mut style = terminal.charts[&7].annotations[0].style.clone();
        style.color = crate::annotations::DEFAULT_LEVEL_COLOR;
        let _ = terminal.update(Message::RestyleAnnotation(7, 5, style.clone()));
        assert_eq!(terminal.charts[&7].drawing_menu, None);
        assert_eq!(terminal.charts[&7].chart.annotations[0].style, style);
        let _ = terminal.update(toggle.clone());
        let _ = terminal.update(Message::SelectAnnotation(7, None));
        let _ = terminal.update(toggle);
        assert_eq!(terminal.charts[&7].drawing_menu, None);
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

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Task;

const CHART_EDITOR_RESULT_ROW_HEIGHT: f32 = 28.0;
const CHART_EDITOR_RESULT_SCROLL_PADDING: f32 = 104.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::preferences_update::hotkeys::keyboard) enum ChartEditorSelectionStep {
    Previous,
    Next,
}

impl TradingTerminal {
    pub(super) fn handle_chart_editor_keyboard(
        &mut self,
        key: iced::keyboard::Key<&str>,
        modifiers: iced::keyboard::Modifiers,
    ) -> Option<Task<Message>> {
        let secondary_editor_id = self.active_chart_secondary_editor_id();
        let editor_id = secondary_editor_id.or_else(|| self.active_chart_editor_id())?;
        let secondary = secondary_editor_id == Some(editor_id);
        if modifiers.control() || modifiers.alt() || modifiers.logo() {
            return None;
        }

        match key {
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                Some(self.move_chart_editor_selection(
                    editor_id,
                    ChartEditorSelectionStep::Next,
                    secondary,
                ))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) if !modifiers.shift() => {
                Some(self.move_chart_editor_selection(
                    editor_id,
                    ChartEditorSelectionStep::Next,
                    secondary,
                ))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                Some(self.move_chart_editor_selection(
                    editor_id,
                    ChartEditorSelectionStep::Previous,
                    secondary,
                ))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) if modifiers.shift() => {
                Some(self.move_chart_editor_selection(
                    editor_id,
                    ChartEditorSelectionStep::Previous,
                    secondary,
                ))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter) => {
                let selected = self.charts.get(&editor_id).is_some_and(|instance| {
                    if secondary {
                        instance.secondary_editor_selected_index.is_some()
                    } else {
                        instance.editor_selected_index.is_some()
                    }
                });
                if selected {
                    return Some(self.update(if secondary {
                        Message::ChartSecondaryEditorSubmit(editor_id)
                    } else {
                        Message::ChartEditorSubmit(editor_id)
                    }));
                }
                None
            }
            _ => None,
        }
    }

    fn move_chart_editor_selection(
        &mut self,
        editor_id: crate::chart_state::ChartId,
        step: ChartEditorSelectionStep,
        secondary: bool,
    ) -> Task<Message> {
        let Some((query, current_index)) = self.charts.get(&editor_id).map(|instance| {
            if secondary {
                (
                    instance.secondary_editor_search_query.trim().to_string(),
                    instance.secondary_editor_selected_index,
                )
            } else {
                (
                    instance.editor_search_query.trim().to_string(),
                    instance.editor_selected_index,
                )
            }
        }) else {
            return Task::none();
        };

        let result_count = self.chart_editor_filtered_symbols(&query).len()
            + usize::from(self.schwab_chart_symbol_candidate(&query).is_some());
        let next_index = next_chart_editor_selection(current_index, result_count, step);
        if let Some(instance) = self.charts.get_mut(&editor_id) {
            if secondary {
                instance.secondary_editor_selected_index = next_index;
            } else {
                instance.editor_selected_index = next_index;
            }
        }

        let Some(index) = next_index else {
            return Task::none();
        };

        let offset_y = (index as f32 * CHART_EDITOR_RESULT_ROW_HEIGHT
            - CHART_EDITOR_RESULT_SCROLL_PADDING)
            .max(0.0);

        if secondary {
            Task::batch([
                Self::scroll_chart_secondary_symbol_search_results_to(editor_id, offset_y),
                Self::focus_chart_secondary_symbol_search_input(editor_id),
            ])
        } else {
            Task::batch([
                Self::scroll_chart_symbol_search_results_to(editor_id, offset_y),
                Self::focus_chart_symbol_search_input(editor_id),
            ])
        }
    }
}

pub(in crate::preferences_update::hotkeys::keyboard) fn next_chart_editor_selection(
    current_index: Option<usize>,
    result_count: usize,
    step: ChartEditorSelectionStep,
) -> Option<usize> {
    if result_count == 0 {
        return None;
    }

    match (current_index.filter(|index| *index < result_count), step) {
        (Some(index), ChartEditorSelectionStep::Next) => {
            Some(index.saturating_add(1).min(result_count - 1))
        }
        (Some(index), ChartEditorSelectionStep::Previous) => Some(index.saturating_sub(1)),
        (None, ChartEditorSelectionStep::Next) => Some(0),
        (None, ChartEditorSelectionStep::Previous) => Some(result_count - 1),
    }
}

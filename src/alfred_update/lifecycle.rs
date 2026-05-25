use crate::alfred_state::AlfredSelectionStep;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

// ---------------------------------------------------------------------------
// Alfred Lifecycle
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn alfred_input_id() -> iced::widget::Id {
        iced::widget::Id::from("alfred_input")
    }

    pub(super) fn open_alfred(&mut self) -> Task<Message> {
        self.close_chart_header_menus();
        self.add_widget_menu_open = false;
        self.layout_menu_open = false;
        self.layout_rename_index = None;
        self.layout_rename_input.clear();
        self.account_picker_open = false;
        self.account_picker_rename_index = None;
        self.alfred.open = true;
        self.alfred.query.clear();
        self.alfred.selected_index = 0;

        iced::widget::operation::focus(Self::alfred_input_id())
    }

    pub(super) fn move_alfred_selection(&mut self, step: AlfredSelectionStep) {
        let result_count = self.alfred_filtered_commands().len();
        if result_count == 0 {
            self.alfred.selected_index = 0;
            return;
        }

        let current = self.alfred.selected_index.min(result_count - 1);
        self.alfred.selected_index = match step {
            AlfredSelectionStep::Previous => current.saturating_sub(1),
            AlfredSelectionStep::Next => current.saturating_add(1).min(result_count - 1),
        };
    }
}

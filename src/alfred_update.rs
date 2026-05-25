use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

mod keyboard;
mod lifecycle;
mod submit;

// ---------------------------------------------------------------------------
// Alfred update
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn update_alfred(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleAlfred => {
                if self.alfred.open {
                    self.alfred.close();
                    Task::none()
                } else {
                    self.open_alfred()
                }
            }
            Message::CloseAlfred => {
                self.alfred.close();
                Task::none()
            }
            Message::AlfredQueryChanged(query) => {
                self.alfred.query = query;
                self.alfred.selected_index = 0;
                Task::none()
            }
            Message::AlfredSelectionMoved(step) => {
                self.move_alfred_selection(step);
                Task::none()
            }
            Message::AlfredSubmit => self.submit_selected_alfred_command(),
            Message::AlfredCommandSelected(id) => self.submit_alfred_command(id),
            _ => Task::none(),
        }
    }
}

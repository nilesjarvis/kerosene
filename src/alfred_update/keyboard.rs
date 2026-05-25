use crate::alfred_state::AlfredSelectionStep;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

// ---------------------------------------------------------------------------
// Alfred Keyboard Handling
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn handle_alfred_keyboard(
        &mut self,
        key: iced::keyboard::Key<&str>,
        modifiers: iced::keyboard::Modifiers,
        status: iced::event::Status,
    ) -> Task<Message> {
        if modifiers.control() || modifiers.alt() || modifiers.logo() {
            return Task::none();
        }

        match key {
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
                self.update(Message::CloseAlfred)
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                self.update(Message::AlfredSelectionMoved(AlfredSelectionStep::Next))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) if !modifiers.shift() => {
                self.update(Message::AlfredSelectionMoved(AlfredSelectionStep::Next))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                self.update(Message::AlfredSelectionMoved(AlfredSelectionStep::Previous))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) if modifiers.shift() => {
                self.update(Message::AlfredSelectionMoved(AlfredSelectionStep::Previous))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter)
                if status == iced::event::Status::Ignored =>
            {
                self.update(Message::AlfredSubmit)
            }
            _ => Task::none(),
        }
    }
}

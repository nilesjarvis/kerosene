use crate::alfred_state::{AlfredCommand, AlfredCommandId};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

mod positions;
mod trading;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Alfred Command Submission
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn submit_selected_alfred_command(&mut self) -> Task<Message> {
        let commands = self.alfred_filtered_commands();
        let Some(command) = selected_command(&commands, self.alfred.selected_index) else {
            self.push_toast("No Alfred matches".to_string(), true);
            return Task::none();
        };

        self.submit_alfred_command(command.id)
    }

    pub(super) fn submit_alfred_command(&mut self, id: AlfredCommandId) -> Task<Message> {
        if id == AlfredCommandId::NaturalLanguageTrading {
            return self.submit_alfred_trade();
        }
        if id == AlfredCommandId::NukePositions {
            return self.submit_alfred_nuke();
        }
        if id == AlfredCommandId::ClosePosition {
            return self.submit_alfred_close_position();
        }

        let Some(command) = self.alfred_command_by_id(id) else {
            self.push_toast("Alfred command is no longer available".to_string(), true);
            return Task::none();
        };

        if !command.enabled {
            self.push_toast(
                command
                    .disabled_reason
                    .unwrap_or_else(|| "Alfred command is not available yet".to_string()),
                true,
            );
            return Task::none();
        }

        let Some(message) = command.message else {
            self.push_toast("Alfred command is not wired yet".to_string(), true);
            return Task::none();
        };

        self.alfred.close();
        self.update(message)
    }
}

fn selected_command(commands: &[AlfredCommand], selected_index: usize) -> Option<&AlfredCommand> {
    let index = selected_index.min(commands.len().checked_sub(1)?);
    commands.get(index)
}

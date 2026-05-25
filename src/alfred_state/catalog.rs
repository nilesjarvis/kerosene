use crate::alfred_state::AlfredCommand;
use crate::app_state::TradingTerminal;

mod availability;
mod widgets;
mod windows;

// ---------------------------------------------------------------------------
// Static Command Catalog
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn alfred_command_catalog(&self) -> Vec<AlfredCommand> {
        let mut commands = self.alfred_widget_commands();
        commands.extend(self.alfred_window_commands());
        commands
    }
}

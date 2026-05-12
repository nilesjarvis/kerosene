use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::{Size, Task, window};

impl TradingTerminal {
    pub(super) fn add_trading_journal_window(&mut self) -> Task<Message> {
        self.add_widget_menu_open = false;
        self.account_picker_open = false;
        self.account_picker_rename_index = None;
        if let Some(id) = self.journal.window_id {
            return Task::batch([
                window::gain_focus(id),
                self.load_journal_for_active_account(false),
            ]);
        }

        let settings = window::Settings {
            size: Size::new(self.journal.width, self.journal.height),
            ..window::Settings::default()
        };
        let (id, task) = window::open(settings);
        self.journal.window_id = Some(id);
        self.journal.open = true;

        let mut tasks = vec![task.map(Message::WindowOpened)];
        tasks.push(self.load_journal_for_active_account(false));

        Task::batch(tasks)
    }
}

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::{Size, Task, window};

impl TradingTerminal {
    pub(super) fn open_wallet_tracker_window(&mut self) -> Task<Message> {
        self.add_widget_menu_open = false;
        self.account_picker_open = false;
        self.account_picker_rename_index = None;
        if let Some(id) = self.wallet_tracker.window_id {
            return window::gain_focus(id);
        }

        let settings = window::Settings {
            size: Size::new(self.wallet_tracker.width, self.wallet_tracker.height),
            ..crate::window_chrome::settings(self.custom_window_chrome_active)
        };
        let (id, task) = window::open(settings);
        self.wallet_tracker.window_id = Some(id);
        self.wallet_tracker.open = true;

        self.queue_wallet_tracker_core_refresh_all();
        let tasks = vec![
            task.map(Message::WindowOpened),
            self.refresh_next_wallet_tracker_core(),
        ];
        self.persist_config();
        Task::batch(tasks)
    }
}

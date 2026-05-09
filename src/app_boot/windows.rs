use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::{Point, Size, Task, window};

impl TradingTerminal {
    pub(super) fn boot_window_tasks(&mut self) -> Vec<Task<Message>> {
        let mut boot_tasks = Vec::new();

        let main_window_settings = window::Settings {
            size: self.main_window_size.unwrap_or(Size::new(1600.0, 960.0)),
            position: self
                .main_window_pos
                .map(window::Position::Specific)
                .unwrap_or_else(|| window::Position::Centered),
            ..window::Settings::default()
        };
        let (main_id, main_open_task) = window::open(main_window_settings);
        self.main_window_id = Some(main_id);
        boot_tasks.push(main_open_task.map(Message::WindowOpened));

        if self.wallet_tracker.open {
            let tracker_settings = window::Settings {
                size: Size::new(self.wallet_tracker.width, self.wallet_tracker.height),
                position: self
                    .wallet_tracker
                    .x
                    .zip(self.wallet_tracker.y)
                    .map(|(x, y)| window::Position::Specific(Point::new(x, y)))
                    .unwrap_or_else(|| window::Position::Centered),
                ..window::Settings::default()
            };
            let (wallet_id, wallet_open_task) = window::open(tracker_settings);
            self.wallet_tracker.window_id = Some(wallet_id);
            boot_tasks.push(wallet_open_task.map(Message::WindowOpened));
            self.queue_wallet_tracker_core_refresh_all();
            boot_tasks.push(self.refresh_next_wallet_tracker_core());
        }

        boot_tasks
    }
}

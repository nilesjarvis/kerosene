use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(crate) fn update_window(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowOpened(_id) => {}
            Message::WindowMoved(id, point) => {
                if Some(id) == self.wallet_tracker.window_id {
                    self.wallet_tracker.x = Some(point.x);
                    self.wallet_tracker.y = Some(point.y);
                    self.persist_config();
                } else if let Some(state) = self.wallet_detail_windows.get_mut(&id) {
                    state.x = Some(point.x);
                    state.y = Some(point.y);
                } else if Some(id) == self.main_window_id {
                    self.main_window_pos = Some(point);
                    self.persist_config();
                }
            }
            Message::WindowClosed(id) => {
                if Some(id) == self.main_window_id {
                    return self.flush_pending_config_save_and_exit();
                }
                if Some(id) == self.settings_window_id {
                    self.settings_window_id = None;
                }
                if Some(id) == self.wallet_tracker.window_id {
                    self.wallet_tracker.window_id = None;
                    self.wallet_tracker.open = false;
                    self.persist_config();
                }
                self.wallet_detail_windows.remove(&id);
                if Some(id) == self.journal.window_id {
                    self.journal.window_id = None;
                    self.journal.open = false;
                }
            }
            Message::WindowResized(id, size) => {
                if Some(id) == self.wallet_tracker.window_id {
                    self.wallet_tracker.width = size.width;
                    self.wallet_tracker.height = size.height;
                    self.persist_config();
                }
                if let Some(state) = self.wallet_detail_windows.get_mut(&id) {
                    state.width = size.width;
                    state.height = size.height;
                }
                if Some(id) == self.journal.window_id {
                    self.journal.width = size.width;
                    self.journal.height = size.height;
                }
            }
            _ => {}
        }

        Task::none()
    }
}

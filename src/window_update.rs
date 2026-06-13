use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::{Task, window};

impl TradingTerminal {
    pub(crate) fn update_window(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowOpened(id) if Some(id) == self.main_window_id => {
                return self.sync_main_window_min_size();
            }
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
                } else if let Some(state) = self.detached_chart_windows.get_mut(&id) {
                    state.x = Some(point.x);
                    state.y = Some(point.y);
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
                if Some(id) == self.screener.window_id {
                    self.screener.window_id = None;
                    self.screener.invalidate_refreshes();
                }
                if Some(id) == self.chart_screenshot_window_id {
                    self.chart_screenshot_window_id = None;
                    self.chart_screenshot = None;
                    self.chart_screenshot_error = None;
                    self.chart_screenshot_capture_in_progress = false;
                    self.chart_screenshot_pending_request_id = None;
                }
                self.pnl_card_windows.remove(&id);
                if self.remove_detached_chart_window_state(id) {
                    self.persist_config();
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
                    self.journal.finish_chart_reveal();
                }
                for twap in self.twap_orders.values_mut() {
                    if twap.window_id == Some(id) {
                        twap.window_id = None;
                    }
                }
                self.advanced_order_history_windows.remove(&id);
            }
            Message::WindowResized(id, size) => {
                if Some(id) == self.wallet_tracker.window_id {
                    self.wallet_tracker.width = size.width;
                    self.wallet_tracker.height = size.height;
                    self.persist_config();
                }
                if Some(id) == self.main_window_id {
                    self.main_window_size = Some(size);
                    self.persist_config();
                    return self.sync_main_window_min_size();
                }
                if let Some(state) = self.detached_chart_windows.get_mut(&id) {
                    state.width = size.width;
                    state.height = size.height;
                    self.persist_config();
                }
                if let Some(state) = self.wallet_detail_windows.get_mut(&id) {
                    state.width = size.width;
                    state.height = size.height;
                }
                if Some(id) == self.journal.window_id {
                    self.journal.width = size.width;
                    self.journal.height = size.height;
                    self.persist_config();
                }
            }
            Message::WindowDrag(id) => {
                return window::drag(id);
            }
            Message::WindowDragResize(id, direction) => {
                return window::drag_resize(id, direction);
            }
            Message::WindowMinimize(id) => {
                return window::minimize(id, true);
            }
            Message::WindowToggleMaximize(id) => {
                return window::toggle_maximize(id);
            }
            Message::WindowClose(id) => {
                return window::close(id);
            }
            _ => {}
        }

        Task::none()
    }
}

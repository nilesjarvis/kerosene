use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::{Size, Task, window};

// ---------------------------------------------------------------------------
// Screenshot Window Lifecycle
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn open_or_focus_chart_screenshot_window(
        &mut self,
        task: Task<Message>,
    ) -> Task<Message> {
        if let Some(id) = self.chart_screenshot_window_id {
            return Task::batch([window::gain_focus(id), task]);
        }

        let settings = window::Settings {
            size: Size::new(720.0, 560.0),
            ..crate::window_chrome::settings(self.custom_window_chrome_active)
        };
        let (id, open_task) = window::open(settings);
        self.chart_screenshot_window_id = Some(id);

        Task::batch([open_task.map(Message::WindowOpened), task])
    }

    pub(super) fn finish_chart_screenshot_error(&mut self, request_id: u64, err: String) {
        if self.chart_screenshot_pending_request_id != Some(request_id) {
            return;
        }

        self.chart_screenshot_pending_request_id = None;
        self.chart_screenshot_capture_in_progress = false;
        self.chart_screenshot_error = Some(err.clone());
        self.push_toast(err, true);
    }
}

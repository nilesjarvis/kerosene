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
            ..crate::window_chrome::settings(
                self.custom_window_chrome_active,
                self.window_background_blur_enabled,
            )
        };
        let (id, open_task) = window::open(settings);
        self.chart_screenshot_window_id = Some(id);

        Task::batch([open_task.map(Message::WindowOpened), task])
    }
}

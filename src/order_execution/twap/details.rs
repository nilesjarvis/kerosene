use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::{Size, Task, window};

// ---------------------------------------------------------------------------
// TWAP Details Window
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn open_twap_details(&mut self, twap_id: u64) -> Task<Message> {
        let Some(twap) = self.twap_orders.get_mut(&twap_id) else {
            return Task::none();
        };
        if let Some(window_id) = twap.window_id {
            return window::gain_focus(window_id);
        }
        let settings = window::Settings {
            size: Size::new(760.0, 560.0),
            ..crate::window_chrome::settings()
        };
        let (window_id, task) = window::open(settings);
        twap.window_id = Some(window_id);
        task.map(Message::WindowOpened)
    }
}

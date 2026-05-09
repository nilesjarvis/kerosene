use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Task;
use std::time::{Duration, Instant};

const NUKE_CONFIRMATION_WINDOW: Duration = Duration::from_secs(5);

impl TradingTerminal {
    pub(crate) fn handle_nuke_positions(&mut self) -> Task<Message> {
        self.close_menu_coin = None;
        let now = Instant::now();
        let armed = nuke_confirmation_is_armed(self.nuke_confirmation, now);
        if !armed {
            self.nuke_confirmation = Some(now);
            self.order_status = Some((
                "NUKE armed: press NUKE again within 5 seconds to close all positions".to_string(),
                true,
            ));
            return Task::none();
        }
        self.nuke_confirmation = None;
        self.execute_nuke_positions()
    }
}

pub(crate) fn nuke_confirmation_is_armed(armed_at: Option<Instant>, now: Instant) -> bool {
    armed_at.is_some_and(|armed_at| now.duration_since(armed_at) <= NUKE_CONFIRMATION_WINDOW)
}

#[cfg(test)]
mod tests {
    use super::{NUKE_CONFIRMATION_WINDOW, nuke_confirmation_is_armed};
    use std::time::{Duration, Instant};

    #[test]
    fn nuke_confirmation_is_only_armed_inside_window() {
        let now = Instant::now();

        assert!(!nuke_confirmation_is_armed(None, now));
        assert!(nuke_confirmation_is_armed(
            Some(now - NUKE_CONFIRMATION_WINDOW),
            now
        ));
        assert!(!nuke_confirmation_is_armed(
            Some(now - NUKE_CONFIRMATION_WINDOW - Duration::from_millis(1)),
            now
        ));
    }
}

use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::session_data_state::{SessionDataId, SessionDataInstance};
use iced::Task;

// ---------------------------------------------------------------------------
// Layout Session Data Restoration
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn restore_layout_session_data(
        &mut self,
        layout: &config::SavedLayout,
    ) -> Task<Message> {
        self.session_data.clear();
        self.next_session_data_id = 0;

        for config in &layout.session_data {
            let symbol = self.visible_session_data_symbol(&config.symbol);
            self.session_data.insert(
                config.id,
                SessionDataInstance::new(config.id, symbol, config.lookback),
            );
            self.next_session_data_id = self.next_session_data_id.max(config.id + 1);
        }

        for id in session_data_pane_ids(&self.panes) {
            if !self.session_data.contains_key(&id) {
                let symbol = self.visible_session_data_symbol("");
                self.session_data
                    .insert(id, SessionDataInstance::new(id, symbol, Default::default()));
                self.next_session_data_id = self.next_session_data_id.max(id + 1);
            }
        }

        self.request_session_data_refresh_all(false)
    }
}

fn session_data_pane_ids(panes: &iced::widget::pane_grid::State<PaneKind>) -> Vec<SessionDataId> {
    panes
        .iter()
        .filter_map(|(_, kind)| {
            if let PaneKind::SessionData(id) = kind {
                Some(*id)
            } else {
                None
            }
        })
        .collect()
}

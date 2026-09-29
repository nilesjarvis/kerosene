use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;
use crate::positioning_state::PositioningInfoInstance;
use iced::Task;

// ---------------------------------------------------------------------------
// Layout Positioning-Info Restoration
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn restore_layout_positioning_infos(
        &mut self,
        layout: &config::SavedLayout,
    ) -> Task<Message> {
        self.positioning_infos.clear();
        self.positioning_info_pending.clear();
        self.next_positioning_info_id = 0;

        for config in &layout.positioning_infos {
            let symbol = self.retained_positioning_symbol(&config.symbol);
            let instance = PositioningInfoInstance::from_config(config, symbol);
            self.positioning_infos.insert(config.id, instance);
            self.next_positioning_info_id = self.next_positioning_info_id.max(config.id + 1);
        }

        self.ensure_positioning_info_pane_instances();

        self.request_positioning_info_refresh_all(false)
    }
}

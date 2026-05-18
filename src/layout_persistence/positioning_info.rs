use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::positioning_state::{PositioningInfoId, PositioningInfoInstance};
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
            let symbol = self.visible_positioning_symbol(&config.symbol);
            let mut instance = PositioningInfoInstance::new(config.id, symbol);
            instance.side = config.side;
            instance.sort_field = config.sort_field;
            instance.sort_direction = config.sort_direction;
            instance.normalize_removed_filters();
            self.positioning_infos.insert(config.id, instance);
            self.next_positioning_info_id = self.next_positioning_info_id.max(config.id + 1);
        }

        let pane_ids: Vec<PositioningInfoId> = self
            .panes
            .iter()
            .filter_map(|(_, kind)| {
                if let PaneKind::PositioningInfo(id) = kind {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        for id in pane_ids {
            if !self.positioning_infos.contains_key(&id) {
                let symbol = self.visible_positioning_symbol("");
                self.positioning_infos
                    .insert(id, PositioningInfoInstance::new(id, symbol));
                self.next_positioning_info_id = self.next_positioning_info_id.max(id + 1);
            }
        }

        self.request_positioning_info_refresh_all(false)
    }
}

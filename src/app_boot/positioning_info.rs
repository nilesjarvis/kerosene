use crate::app_state::TradingTerminal;
use crate::config::KeroseneConfig;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::positioning_state::{PositioningInfoId, PositioningInfoInstance};
use iced::Task;
use std::collections::HashSet;

impl TradingTerminal {
    pub(super) fn boot_positioning_info_instances(
        &mut self,
        cfg: &KeroseneConfig,
        muted_tickers: &HashSet<String>,
    ) {
        for config in &cfg.positioning_infos {
            let symbol = if Self::key_matches_muted_tickers(&[], muted_tickers, &config.symbol) {
                self.active_symbol.clone()
            } else {
                config.symbol.clone()
            };
            let mut instance =
                PositioningInfoInstance::new(config.id, self.visible_positioning_symbol(&symbol));
            instance.page = config.page;
            instance.side = config.side;
            instance.sort_field = config.sort_field;
            instance.sort_direction = config.sort_direction;
            instance.entry_min_input = config.entry_min.clone();
            instance.entry_max_input = config.entry_max.clone();
            instance.change_timeframe = config.change_timeframe;
            instance.change_sort_field = config.change_sort_field;
            instance.change_sort_direction = config.change_sort_direction;
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
    }

    pub(super) fn boot_positioning_info_tasks(&mut self) -> Task<Message> {
        self.request_positioning_info_refresh_all(false)
    }
}

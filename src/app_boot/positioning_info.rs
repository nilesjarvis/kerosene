use crate::app_state::TradingTerminal;
use crate::config::KeroseneConfig;
use crate::message::Message;
use crate::positioning_state::PositioningInfoInstance;
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
            let instance = PositioningInfoInstance::from_config(
                config,
                self.retained_positioning_symbol(&symbol),
            );
            self.positioning_infos.insert(config.id, instance);
            self.next_positioning_info_id = self.next_positioning_info_id.max(config.id + 1);
        }

        self.ensure_positioning_info_pane_instances();
    }

    pub(super) fn boot_positioning_info_tasks(&mut self) -> Task<Message> {
        self.request_positioning_info_refresh_all(false)
    }
}

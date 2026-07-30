use crate::app_state::TradingTerminal;
use crate::config::KeroseneConfig;
use crate::pane_state::PaneKind;
use crate::session_data_state::SessionDataInstance;
use std::collections::HashSet;

impl TradingTerminal {
    pub(super) fn boot_session_data_instances(
        &mut self,
        cfg: &KeroseneConfig,
        muted_tickers: &HashSet<String>,
    ) {
        for config in &cfg.session_data {
            let symbol = if Self::key_matches_muted_tickers(&[], muted_tickers, &config.symbol) {
                self.active_symbol.clone()
            } else {
                self.visible_session_data_symbol(&config.symbol)
            };
            self.session_data.insert(
                config.id,
                SessionDataInstance::new(config.id, symbol, config.lookback),
            );
            self.next_session_data_id = self.next_session_data_id.max(config.id + 1);
        }

        let pane_ids = self
            .workspace_pane_kinds()
            .filter_map(|(_, _, kind)| match kind {
                PaneKind::SessionData(id) => Some(*id),
                _ => None,
            })
            .collect::<Vec<_>>();
        for id in pane_ids {
            if !self.session_data.contains_key(&id) {
                self.session_data.insert(
                    id,
                    SessionDataInstance::new(id, self.active_symbol.clone(), Default::default()),
                );
                self.next_session_data_id = self.next_session_data_id.max(id + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{KeroseneConfig, PaneKindConfig, PaneLayoutConfig, SessionDataConfig};
    use crate::session_data_state::SessionDataLookback;

    #[test]
    fn boot_creates_session_data_instance_for_persisted_pane() {
        let (terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig {
            pane_layout: Some(PaneLayoutConfig::Leaf(PaneKindConfig::SessionData {
                id: 9,
            })),
            session_data: vec![SessionDataConfig {
                id: 9,
                symbol: "@107".to_string(),
                lookback: SessionDataLookback::EightWeeks,
            }],
            ..KeroseneConfig::default()
        });

        let instance = terminal
            .session_data
            .get(&9)
            .expect("persisted session data instance");
        assert_eq!(instance.symbol, "@107");
        assert_eq!(instance.lookback, SessionDataLookback::EightWeeks);
    }
}

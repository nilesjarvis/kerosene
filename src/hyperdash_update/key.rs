use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;
use zeroize::{Zeroize, Zeroizing};

impl TradingTerminal {
    pub(super) fn update_hyperdash_key(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::HyperdashKeyInputChanged(value) => {
                self.hyperdash_key_input.zeroize();
                self.hyperdash_key_input = value.into_zeroizing().into();
            }
            Message::SaveHyperdashKey => {
                let previous_key = Zeroizing::new(self.hyperdash_api_key.trim().to_string());
                let next_key = Zeroizing::new(self.hyperdash_key_input.trim().to_string());
                if !self.persist_hyperdash_secret_from_key(next_key.as_str()) {
                    return Task::none();
                }

                self.hyperdash_api_key.zeroize();
                self.hyperdash_api_key = next_key.as_str().to_string().into();
                let hyperdash_key_changed = previous_key.as_str() != next_key.as_str();
                if hyperdash_key_changed {
                    self.bump_hyperdash_key_generation();
                }
                self.persist_config();
                let heatmap_ids: Vec<ChartId> = self
                    .charts
                    .iter()
                    .filter(|(_, inst)| inst.show_heatmap && !inst.symbol.is_empty())
                    .map(|(id, _)| *id)
                    .collect();
                let liquidation_ids: Vec<ChartId> = self
                    .charts
                    .iter()
                    .filter(|(_, inst)| inst.show_liquidations && !inst.symbol.is_empty())
                    .map(|(id, _)| *id)
                    .collect();
                let distribution_open =
                    self.pane_is_open(|kind| matches!(kind, PaneKind::LiquidationsDistribution));
                self.liquidation_pending_charts.clear();
                for id in &liquidation_ids {
                    if let Some(instance) = self.charts.get_mut(id) {
                        instance.liquidation_fetching = false;
                        instance.liquidation_pending_key = None;
                    }
                }
                if self.hyperdash_api_key.is_empty() {
                    for id in heatmap_ids {
                        if let Some(instance) = self.charts.get_mut(&id) {
                            instance.heatmap_status = Some((
                                "Add HyperDash key in Settings > Integrations".to_string(),
                                true,
                            ));
                        }
                    }
                    for id in liquidation_ids {
                        if let Some(instance) = self.charts.get_mut(&id) {
                            Self::clear_liquidation_display(instance);
                            instance.liquidation_status = Some((
                                "Add HyperDash key in Settings > Integrations".to_string(),
                                true,
                            ));
                            instance.chart.candle_cache.clear();
                        }
                    }
                    if distribution_open {
                        let _ = self.request_liquidation_distribution_refresh(true);
                    }
                    return self.request_positioning_info_refresh_all(true);
                }
                let mut tasks: Vec<Task<Message>> = heatmap_ids
                    .into_iter()
                    .map(|id| self.maybe_fetch_heatmap(id))
                    .collect();
                tasks.extend(
                    liquidation_ids
                        .into_iter()
                        .map(|id| self.maybe_fetch_liquidations(id)),
                );
                if distribution_open {
                    tasks.push(self.request_liquidation_distribution_refresh(true));
                }
                tasks.push(self.request_positioning_info_refresh_all(true));
                if !tasks.is_empty() {
                    return Task::batch(tasks);
                }
            }
            _ => {}
        }

        Task::none()
    }

    pub(crate) fn invalidate_hyperdash_chart_requests_for_key_change(&mut self) {
        let heatmap_ids: Vec<ChartId> = self
            .charts
            .iter()
            .filter(|(_, inst)| {
                inst.show_heatmap
                    || inst.heatmap_fetching
                    || inst.heatmap_last_fetch.is_some()
                    || inst.heatmap_data.is_some()
            })
            .map(|(id, _)| *id)
            .collect();
        self.invalidate_heatmap_requests_for_hyperdash_key_change(&heatmap_ids);

        self.liquidation_pending_charts.clear();
        for instance in self.charts.values_mut() {
            instance.liquidation_fetching = false;
            instance.liquidation_pending_key = None;
        }
    }

    fn invalidate_heatmap_requests_for_hyperdash_key_change(&mut self, heatmap_ids: &[ChartId]) {
        self.heatmap_pending_charts.clear();
        self.heatmap_data_cache.clear();
        self.heatmap_data_cache_order.clear();

        for id in heatmap_ids {
            if let Some(instance) = self.charts.get_mut(id) {
                instance.heatmap_fetching = false;
                instance.heatmap_last_fetch = None;
                instance.heatmap_status = None;
                Self::clear_heatmap_display(instance);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app_state::{TradingTerminal, sensitive_string};
    use crate::chart_state::ChartInstance;
    use crate::config;
    use crate::hyperdash_api::{HeatmapFetchParams, LiquidationHeatmap, LiquidationLevel};
    use crate::timeframe::Timeframe;

    #[test]
    fn hyperdash_generation_bump_invalidates_chart_pending_state_and_cache() {
        let (mut terminal, _) = TradingTerminal::boot();
        let chart_id = 7;
        populate_hyperdash_pending_state(&mut terminal, chart_id);

        terminal.bump_hyperdash_key_generation();

        assert_eq!(terminal.hyperdash_key_generation, 1);
        assert!(terminal.heatmap_pending_charts.is_empty());
        assert!(terminal.heatmap_data_cache.is_empty());
        assert!(terminal.heatmap_data_cache_order.is_empty());
        assert!(terminal.liquidation_pending_charts.is_empty());

        let instance = terminal
            .charts
            .get(&chart_id)
            .expect("chart should remain registered");
        assert!(!instance.heatmap_fetching);
        assert!(instance.heatmap_last_fetch.is_none());
        assert!(instance.heatmap_status.is_none());
        assert!(instance.heatmap_data.is_none());
        assert!(!instance.liquidation_fetching);
        assert!(instance.liquidation_pending_key.is_none());
        assert!(instance.liquidation_data.is_some());
    }

    #[test]
    fn hyperdash_save_failure_preserves_live_key_generation_and_chart_caches() {
        let (mut terminal, _) = TradingTerminal::boot();
        configure_encrypted_hyperdash_key(&mut terminal, "old-hyper", false);
        terminal.hyperdash_api_key = sensitive_string("old-hyper");
        terminal.hyperdash_key_input = sensitive_string("new-hyper");
        terminal.hyperdash_key_generation = 4;
        let chart_id = 7;
        let (cache_key, liquidation_key) =
            populate_hyperdash_pending_state(&mut terminal, chart_id);
        terminal.config_save_due_at = None;

        let _task = terminal.update_hyperdash_key(Message::SaveHyperdashKey);

        assert_eq!(terminal.hyperdash_api_key.as_str(), "old-hyper");
        assert_eq!(terminal.hyperdash_key_input.as_str(), "new-hyper");
        assert_eq!(terminal.hyperdash_key_generation, 4);
        assert!(terminal.heatmap_pending_charts.contains_key(&cache_key));
        assert!(terminal.heatmap_data_cache.contains_key(&cache_key));
        assert_eq!(
            terminal.heatmap_data_cache_order.iter().next(),
            Some(&cache_key)
        );
        assert!(
            terminal
                .liquidation_pending_charts
                .contains_key(&liquidation_key)
        );
        assert_chart_pending_state_preserved(&terminal, chart_id);
        assert!(terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_none());
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Unlock encrypted credentials"));
    }

    #[test]
    fn hyperdash_clear_failure_does_not_clear_liquidation_or_heatmap_state() {
        let (mut terminal, _) = TradingTerminal::boot();
        configure_encrypted_hyperdash_key(&mut terminal, "old-hyper", false);
        terminal.hyperdash_api_key = sensitive_string("old-hyper");
        terminal.hyperdash_key_input = sensitive_string("");
        terminal.hyperdash_key_generation = 4;
        let chart_id = 7;
        let (cache_key, liquidation_key) =
            populate_hyperdash_pending_state(&mut terminal, chart_id);
        terminal.config_save_due_at = None;

        let _task = terminal.update_hyperdash_key(Message::SaveHyperdashKey);

        assert_eq!(terminal.hyperdash_api_key.as_str(), "old-hyper");
        assert_eq!(terminal.hyperdash_key_generation, 4);
        assert!(terminal.heatmap_pending_charts.contains_key(&cache_key));
        assert!(
            terminal
                .liquidation_pending_charts
                .contains_key(&liquidation_key)
        );
        assert_chart_pending_state_preserved(&terminal, chart_id);
        let payload = config::decrypt_secrets(
            terminal
                .encrypted_secrets
                .as_ref()
                .expect("encrypted secrets should remain present"),
            &terminal.encrypted_secret_password,
        )
        .expect("encrypted secrets should decrypt");
        assert_eq!(payload.global_hyperdash_api_key(), "old-hyper");
        assert!(terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_none());
    }

    #[test]
    fn hyperdash_save_commits_after_encrypted_persistence_succeeds() {
        let (mut terminal, _) = TradingTerminal::boot();
        configure_encrypted_hyperdash_key(&mut terminal, "old-hyper", true);
        terminal.hyperdash_api_key = sensitive_string("old-hyper");
        terminal.hyperdash_key_input = sensitive_string("  new-hyper  ");
        terminal.hyperdash_key_generation = 4;
        let chart_id = 7;
        populate_hyperdash_pending_state(&mut terminal, chart_id);
        terminal.config_save_due_at = None;

        let _task = terminal.update_hyperdash_key(Message::SaveHyperdashKey);

        assert_eq!(terminal.hyperdash_api_key.as_str(), "new-hyper");
        assert_eq!(terminal.hyperdash_key_generation, 5);
        assert!(terminal.heatmap_pending_charts.is_empty());
        assert!(terminal.heatmap_data_cache.is_empty());
        assert!(terminal.heatmap_data_cache_order.is_empty());
        assert!(terminal.liquidation_pending_charts.is_empty());
        let payload = config::decrypt_secrets(
            terminal
                .encrypted_secrets
                .as_ref()
                .expect("encrypted secrets should be rewritten"),
            &terminal.encrypted_secret_password,
        )
        .expect("encrypted secrets should decrypt");
        assert_eq!(payload.global_hydromancer_api_key(), "hydro-key");
        assert_eq!(payload.global_hyperdash_api_key(), "new-hyper");
        assert_eq!(payload.global_x_bearer_token(), "x-token");
        assert!(!terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_some());
    }

    fn configure_encrypted_hyperdash_key(
        terminal: &mut TradingTerminal,
        hyperdash_key: &str,
        unlocked: bool,
    ) {
        terminal.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
        terminal.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
        terminal.encrypted_secret_password = sensitive_string("test-password");
        terminal.encrypted_secrets = Some(
            config::encrypt_secrets(
                &config::SecretPayload::from_credentials(
                    &[],
                    "hydro-key",
                    hyperdash_key,
                    "x-token",
                ),
                &terminal.encrypted_secret_password,
            )
            .expect("test encrypted payload"),
        );
        terminal.encrypted_secrets_unlocked = unlocked;
        terminal.hydromancer_api_key = sensitive_string("hydro-key");
        terminal.x_feed.bearer_token = sensitive_string("x-token");
        terminal.secret_migration_save_blocked = false;
        terminal.secret_store_status = None;
    }

    fn populate_hyperdash_pending_state(
        terminal: &mut TradingTerminal,
        chart_id: ChartId,
    ) -> (String, String) {
        let cache_key = "BTC:1.00000000:2.00000000:10:20".to_string();
        let liquidation_key = "BTC:0.00000000:200.00000000:20".to_string();
        let heatmap = LiquidationHeatmap {
            rects: Vec::new(),
            max_abs_usd: 0.0,
        };
        let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
        instance.show_heatmap = true;
        instance.show_liquidations = true;
        instance.heatmap_fetching = true;
        instance.heatmap_last_fetch = Some(HeatmapFetchParams {
            coin: "BTC".to_string(),
            min_price: 1.0,
            max_price: 2.0,
            start_time: 10,
            end_time: 20,
        });
        instance.heatmap_data = Some(heatmap.clone());
        instance.liquidation_fetching = true;
        instance.liquidation_pending_key = Some(liquidation_key.clone());
        instance.liquidation_data = Some(LiquidationLevel {
            coin: "BTC".to_string(),
            min: 0.0,
            max: 200.0,
            liquidations: Vec::new(),
        });

        terminal.charts.insert(chart_id, instance);
        terminal
            .heatmap_pending_charts
            .insert(cache_key.clone(), vec![chart_id]);
        terminal
            .liquidation_pending_charts
            .insert(liquidation_key.clone(), vec![chart_id]);
        terminal
            .heatmap_data_cache
            .insert(cache_key.clone(), heatmap);
        terminal
            .heatmap_data_cache_order
            .push_back(cache_key.clone());

        (cache_key, liquidation_key)
    }

    fn assert_chart_pending_state_preserved(terminal: &TradingTerminal, chart_id: ChartId) {
        let instance = terminal
            .charts
            .get(&chart_id)
            .expect("chart should remain registered");
        assert!(instance.heatmap_fetching);
        assert!(instance.heatmap_last_fetch.is_some());
        assert!(instance.heatmap_data.is_some());
        assert!(instance.liquidation_fetching);
        assert!(instance.liquidation_pending_key.is_some());
        assert!(instance.liquidation_data.is_some());
    }
}

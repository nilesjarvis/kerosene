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
fn key_invalidation_preserves_inactive_heatmaps_and_clears_all_liquidation_waiters() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.charts.clear();
    terminal.hyperdash_key_generation = u64::MAX;
    for mask in 0..16 {
        populate_hyperdash_pending_state(&mut terminal, mask);
        let instance = terminal.charts.get_mut(&mask).expect("fixture chart");
        instance.show_heatmap = mask & 1 != 0;
        instance.heatmap_fetching = mask & 2 != 0;
        if mask & 4 == 0 {
            instance.heatmap_last_fetch = None;
        }
        if mask & 8 == 0 {
            instance.heatmap_data = None;
        }
        instance.heatmap_status = Some(("existing heatmap status".to_string(), true));
        instance.chart.heatmap_max_usd = 42.0;
        instance.show_liquidations = false;
        instance.liquidation_status = Some(("existing liquidation status".to_string(), false));
    }

    terminal.bump_hyperdash_key_generation();

    assert_eq!(terminal.hyperdash_key_generation, 0);
    assert!(terminal.heatmap_pending_charts.is_empty());
    assert!(terminal.heatmap_data_cache.is_empty());
    assert!(terminal.heatmap_data_cache_order.is_empty());
    assert!(terminal.liquidation_pending_charts.is_empty());
    for (mask, instance) in &terminal.charts {
        assert_eq!(instance.show_heatmap, mask & 1 != 0);
        assert!(!instance.heatmap_fetching);
        assert!(instance.heatmap_last_fetch.is_none());
        assert!(instance.heatmap_data.is_none());
        if *mask == 0 {
            assert_eq!(instance.chart.heatmap_max_usd, 42.0);
            assert_eq!(
                instance.heatmap_status,
                Some(("existing heatmap status".to_string(), true))
            );
        } else {
            assert_eq!(instance.chart.heatmap_max_usd, 0.0);
            assert!(instance.heatmap_status.is_none());
        }
        assert!(!instance.liquidation_fetching);
        assert!(instance.liquidation_pending_key.is_none());
        assert!(instance.liquidation_data.is_some());
        assert_eq!(
            instance.liquidation_status,
            Some(("existing liquidation status".to_string(), false))
        );
    }
}

#[test]
fn hyperdash_save_failure_preserves_live_key_generation_and_chart_caches() {
    let (mut terminal, _) = TradingTerminal::boot();
    configure_encrypted_hyperdash_key(&mut terminal, "old-hyper", false);
    terminal.hyperdash_api_key = sensitive_string("old-hyper");
    terminal.hyperdash_key_input = sensitive_string("new-hyper");
    terminal.hyperdash_key_generation = 4;
    let chart_id = 7;
    let (cache_key, liquidation_key) = populate_hyperdash_pending_state(&mut terminal, chart_id);
    terminal.config_save_due_at = None;
    terminal.agent.runtime_connected = true;
    let runtime_generation = terminal.agent.runtime_generation;

    let _task = terminal.update_hyperdash_key(Message::SaveHyperdashKey);

    assert_eq!(terminal.hyperdash_api_key.as_str(), "old-hyper");
    assert_eq!(terminal.hyperdash_key_input.as_str(), "new-hyper");
    assert_eq!(terminal.hyperdash_key_generation, 4);
    assert!(terminal.agent.runtime_connected);
    assert_eq!(terminal.agent.runtime_generation, runtime_generation);
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
    let (cache_key, liquidation_key) = populate_hyperdash_pending_state(&mut terminal, chart_id);
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
    assert!(!terminal.secret_migration_save_blocked);
    assert!(terminal.config_save_due_at.is_some());
}

#[test]
fn hyperdash_key_rotation_invalidates_running_assistant_runtime() {
    let (mut terminal, _) = TradingTerminal::boot();
    configure_encrypted_hyperdash_key(&mut terminal, "old-hyper", true);
    terminal.hyperdash_api_key = sensitive_string("old-hyper");
    terminal.hyperdash_key_input = sensitive_string("new-hyper");
    terminal.assistant_provider = crate::config::AssistantProvider::OpenRouter;
    terminal.agent.runtime_connected = true;
    let runtime_generation = terminal.agent.runtime_generation;

    let _task = terminal.update_hyperdash_key(Message::SaveHyperdashKey);

    assert_eq!(terminal.hyperdash_api_key.as_str(), "new-hyper");
    assert!(
        !terminal.agent.runtime_connected,
        "rotated HyperDash key must evict the Pi child holding the old key in its env"
    );
    assert_eq!(terminal.agent.runtime_generation, runtime_generation + 1);
}

#[test]
fn hyperdash_key_rotation_invalidates_starting_assistant_runtime() {
    for next_key in ["new-hyper", ""] {
        let (mut terminal, _) = TradingTerminal::boot();
        configure_encrypted_hyperdash_key(&mut terminal, "old-hyper", true);
        terminal.hyperdash_api_key = sensitive_string("old-hyper");
        terminal.hyperdash_key_input = sensitive_string(next_key);
        terminal.assistant_provider = crate::config::AssistantProvider::LlamaCpp;
        terminal.agent.status = crate::agent_state::AgentStatus::Starting;
        terminal.agent.pending_prompt = Some("test".to_string().into());
        let generation = terminal.agent.runtime_generation;

        let _task = terminal.update_hyperdash_key(Message::SaveHyperdashKey);

        assert_eq!(terminal.agent.runtime_generation, generation + 1);
        assert!(terminal.agent.pending_prompt.is_none());
        let _task = terminal.update(Message::AgentRuntimeEvent(
            crate::agent_runtime::AgentRuntimeEvent::Ready { generation },
        ));
        assert!(!terminal.agent.runtime_connected);
    }
}

#[test]
fn saving_unchanged_hyperdash_key_preserves_assistant_runtime() {
    let (mut terminal, _) = TradingTerminal::boot();
    configure_encrypted_hyperdash_key(&mut terminal, "old-hyper", true);
    terminal.hyperdash_api_key = sensitive_string("old-hyper");
    terminal.hyperdash_key_input = sensitive_string("  old-hyper  ");
    terminal.agent.runtime_connected = true;
    let generation = terminal.agent.runtime_generation;

    let _task = terminal.update_hyperdash_key(Message::SaveHyperdashKey);

    assert!(terminal.agent.runtime_connected);
    assert_eq!(terminal.agent.runtime_generation, generation);
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
            &config::SecretPayload::from_credentials(&[], "hydro-key", hyperdash_key),
            &terminal.encrypted_secret_password,
        )
        .expect("test encrypted payload"),
    );
    terminal.encrypted_secrets_unlocked = unlocked;
    terminal.hydromancer_api_key = sensitive_string("hydro-key");
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

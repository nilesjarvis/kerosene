use super::{
    CONFIG_SAVE_DEBOUNCE, ConfigSaveCompletionAction, config_save_completion_action,
    config_save_is_due, config_save_should_start,
};
use crate::app_state::TradingTerminal;
use crate::config::{
    AccountProfile, AxisConfig, KeroseneConfig, OrderBookConfig, PaneKindConfig, PaneLayoutConfig,
    SavedLayout, SecretPayload, default_tick_size,
};
use crate::helpers::valid_book_tick_size;
use std::time::{Duration, Instant};

fn future_pane_layout() -> PaneLayoutConfig {
    PaneLayoutConfig::Split {
        axis: AxisConfig::Vertical,
        ratio: 0.5,
        a: Box::new(PaneLayoutConfig::Leaf(PaneKindConfig::Chart {
            chart_id: 0,
        })),
        b: Box::new(PaneLayoutConfig::Leaf(PaneKindConfig::Unknown(
            serde_json::json!({
                "FuturePane": {
                    "id": 9,
                    "label": "newer-version"
                }
            }),
        ))),
    }
}

#[test]
fn config_save_due_check_waits_until_debounce_deadline() {
    let now = Instant::now();
    let due_at = now + CONFIG_SAVE_DEBOUNCE;

    assert!(!config_save_is_due(None, now));
    assert!(!config_save_is_due(
        Some(due_at),
        now + Duration::from_millis(100)
    ));
    assert!(config_save_is_due(Some(due_at), due_at));
    assert!(config_save_is_due(
        Some(due_at),
        due_at + Duration::from_secs(1)
    ));
}

#[test]
fn config_save_start_waits_for_in_flight_write() {
    let now = Instant::now();
    let due_at = now - Duration::from_millis(1);

    assert!(config_save_should_start(Some(due_at), false, now));
    assert!(!config_save_should_start(Some(due_at), true, now));
    assert!(!config_save_should_start(None, false, now));
}

#[test]
fn config_save_completion_prioritizes_pending_exit_save() {
    // A pending debounced save runs before the exit decision regardless
    // of the just-completed save's success — the user's most-recent
    // changes haven't hit disk yet.
    assert_eq!(
        config_save_completion_action(true, true, true),
        ConfigSaveCompletionAction::SavePending
    );
    assert_eq!(
        config_save_completion_action(true, true, false),
        ConfigSaveCompletionAction::SavePending
    );
}

#[test]
fn config_save_completion_exits_only_after_a_successful_save() {
    assert_eq!(
        config_save_completion_action(true, false, true),
        ConfigSaveCompletionAction::Exit
    );
}

#[test]
fn config_save_completion_blocks_exit_when_final_save_failed() {
    // Exit requested + nothing pending + the just-completed save returned
    // Err → stay open so account layout, muted tickers, hotkeys, presets,
    // etc. aren't silently dropped.
    assert_eq!(
        config_save_completion_action(true, false, false),
        ConfigSaveCompletionAction::BlockExitOnError
    );
}

#[test]
fn pending_exit_flag_remains_armed_while_existing_save_finishes() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.config_save_in_flight = true;

    let task = terminal.flush_pending_config_save_and_exit();

    assert_eq!(task.units(), 0);
    assert!(terminal.config_save_exit_requested);
    assert!(terminal.config_save_in_flight);
}

#[test]
fn successful_exit_keeps_automation_fence_armed_until_runtime_exits() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.config_save_exit_requested = true;
    terminal.config_save_in_flight = true;

    let task = terminal.handle_config_save_result(Ok(()));

    assert_eq!(task.units(), 1);
    assert!(terminal.config_save_exit_requested);
    assert!(!terminal.config_save_in_flight);
}

#[test]
fn exit_without_pending_save_keeps_automation_fence_armed() {
    let mut terminal = TradingTerminal::boot().0;

    let task = terminal.flush_pending_config_save_and_exit();

    assert_eq!(task.units(), 1);
    assert!(terminal.config_save_exit_requested);
}

#[test]
fn failed_exit_save_leaves_immediate_retry_pending() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.config_save_exit_requested = true;
    terminal.config_save_due_at = None;

    let _task = terminal.handle_config_save_result(Err("disk full".to_string()));

    assert!(!terminal.config_save_exit_requested);
    assert!(terminal.config_save_due_at.is_some());
    assert!(config_save_should_start(
        terminal.config_save_due_at,
        terminal.config_save_in_flight,
        Instant::now()
    ));
}

#[test]
fn config_save_failure_status_redacts_sensitive_text() {
    let mut terminal = TradingTerminal::boot().0;

    let _task =
        terminal.handle_config_save_result(Err("write failed: api_key=config-secret".to_string()));

    let (status, is_error) = terminal
        .secret_store_status
        .as_ref()
        .expect("config save failure status");
    assert!(*is_error);
    assert!(status.contains("api_key=<redacted>"));
    assert!(!status.contains("config-secret"));
}

#[test]
fn config_save_completion_does_nothing_when_exit_was_not_requested() {
    assert_eq!(
        config_save_completion_action(false, true, true),
        ConfigSaveCompletionAction::None
    );
    assert_eq!(
        config_save_completion_action(false, false, true),
        ConfigSaveCompletionAction::None
    );
    assert_eq!(
        config_save_completion_action(false, false, false),
        ConfigSaveCompletionAction::None
    );
}

#[test]
fn secret_migration_failure_blocks_debounced_config_save() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.secret_migration_save_blocked = true;

    terminal.persist_config();

    assert!(terminal.config_save_due_at.is_none());
    let (status, is_error) = terminal
        .secret_store_status
        .as_ref()
        .expect("blocked save should set status");
    assert!(*is_error);
    assert!(status.contains("Config save paused"));
}

#[test]
fn secret_migration_failure_blocks_immediate_config_save() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.secret_migration_save_blocked = true;
    let save_called = std::cell::Cell::new(false);

    let result = terminal.persist_config_immediately_with(|_| {
        save_called.set(true);
        Ok(())
    });

    assert!(result.is_err());
    assert!(!save_called.get());
    let (status, is_error) = terminal
        .secret_store_status
        .as_ref()
        .expect("blocked save should set status");
    assert!(*is_error);
    assert!(status.contains("Config save paused"));
}

#[test]
fn secret_rollback_immediate_save_bypasses_secret_migration_block_without_clearing_it() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.secret_migration_save_blocked = true;
    terminal.secret_store_status = Some(("Keychain update failed".to_string(), true));
    terminal.accounts = vec![crate::config::AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: String::new().into(),
        hydromancer_api_key: String::new().into(),
    }];
    let mut saved_wallet_address = None;

    terminal
        .persist_config_immediately_for_secret_migration_rollback_with(|cfg| {
            saved_wallet_address = cfg
                .accounts
                .first()
                .map(|profile| profile.wallet_address.clone());
            Ok(())
        })
        .expect("rollback snapshot should bypass secret migration save block");

    assert_eq!(
        saved_wallet_address.as_deref(),
        Some("0xabc0000000000000000000000000000000000000")
    );
    assert!(terminal.secret_migration_save_blocked);
    assert_eq!(
        terminal.secret_store_status,
        Some(("Keychain update failed".to_string(), true))
    );
}

#[test]
fn config_save_snapshot_persists_valid_fallback_book_tick_size() {
    let mut terminal = TradingTerminal::boot().0;
    let mut persisted_tick = None;

    terminal
        .persist_config_immediately_with(|cfg| {
            persisted_tick = Some(cfg.book_tick_size);
            Ok(())
        })
        .expect("config snapshot should save");

    let tick = persisted_tick.expect("save closure should receive config");
    assert!(valid_book_tick_size(tick));
    assert_eq!(tick, default_tick_size());
}

#[test]
fn config_save_snapshot_persists_app_onboarding_dismissal() {
    let cfg = KeroseneConfig {
        app_onboarding_dismissed: true,
        ..KeroseneConfig::default()
    };
    let mut terminal = TradingTerminal::boot_from_config(cfg).0;
    let mut persisted_dismissed = None;

    terminal
        .persist_config_immediately_with(|cfg| {
            persisted_dismissed = Some(cfg.app_onboarding_dismissed);
            Ok(())
        })
        .expect("config snapshot should save");

    assert_eq!(persisted_dismissed, Some(true));
}

#[test]
fn config_save_snapshot_clears_account_secret_fields_without_mutating_runtime() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.accounts = vec![AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: "agent-secret".to_string().into(),
        hydromancer_api_key: "profile-hydro-secret".to_string().into(),
    }];
    let mut saved_accounts = None;

    terminal
        .persist_config_immediately_with(|cfg| {
            saved_accounts = Some(cfg.accounts.clone());
            Ok(())
        })
        .expect("config snapshot should save");

    let saved_accounts = saved_accounts.expect("save closure should receive config accounts");
    assert_eq!(saved_accounts.len(), 1);
    assert_eq!(saved_accounts[0].secret_id, "acct-a");
    assert_eq!(
        saved_accounts[0].wallet_address,
        "0xabc0000000000000000000000000000000000000"
    );
    assert!(saved_accounts[0].agent_key.is_empty());
    assert!(saved_accounts[0].hydromancer_api_key.is_empty());

    assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-secret");
    assert_eq!(
        terminal.accounts[0].hydromancer_api_key.as_str(),
        "profile-hydro-secret"
    );
}

#[test]
fn persisted_accounts_snapshot_still_feeds_credential_payloads() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.accounts = vec![AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: "agent-secret".to_string().into(),
        hydromancer_api_key: String::new().into(),
    }];

    let accounts = terminal.persisted_accounts_snapshot();
    let payload = SecretPayload::from_credentials(&accounts, "global-hydro", "global-hyper");

    assert_eq!(payload.profile_agent_key("acct-a"), Some("agent-secret"));
    assert_eq!(payload.global_hydromancer_api_key(), "global-hydro");
    assert_eq!(payload.global_hyperdash_api_key(), "global-hyper");
}

#[test]
fn config_save_snapshot_normalizes_legacy_saved_layout_book_ticks() {
    let mut terminal = TradingTerminal::boot().0;
    let mut legacy_layout = terminal.saved_layout_snapshot("Legacy".to_string());
    legacy_layout.book_tick_size = 0.0;
    legacy_layout.order_books = vec![
        serde_json::from_str::<OrderBookConfig>(r#"{"id":7}"#)
            .expect("minimal legacy order book config should deserialize"),
    ];
    terminal.saved_layouts = vec![legacy_layout];

    let mut saved_layout_ticks = None;
    terminal
        .persist_config_immediately_with(|cfg| {
            saved_layout_ticks = cfg.saved_layouts.first().map(|layout| {
                (
                    layout.book_tick_size,
                    layout.order_books.first().map(|book| book.tick_size),
                )
            });
            Ok(())
        })
        .expect("config snapshot should save");

    assert_eq!(
        saved_layout_ticks,
        Some((default_tick_size(), Some(default_tick_size())))
    );
}

#[test]
fn config_save_snapshot_preserves_loaded_unknown_active_pane_layout_when_runtime_matches() {
    let future_layout = future_pane_layout();
    let cfg = KeroseneConfig {
        pane_layout: Some(future_layout.clone()),
        ..KeroseneConfig::default()
    };
    let mut terminal = TradingTerminal::boot_from_config(cfg).0;
    let mut saved_pane_layout = None;

    terminal
        .persist_config_immediately_with(|cfg| {
            saved_pane_layout = cfg.pane_layout.clone();
            Ok(())
        })
        .expect("config snapshot should save");

    assert_eq!(saved_pane_layout, Some(future_layout));
}

#[test]
fn config_save_snapshot_preserves_unknown_saved_layout_pane_layouts() {
    let future_layout = future_pane_layout();
    let saved_layout: SavedLayout = serde_json::from_value(serde_json::json!({
        "name": "future",
        "pane_layout": serde_json::to_value(&future_layout).expect("future layout should serialize")
    }))
    .expect("future saved layout should deserialize");
    let cfg = KeroseneConfig {
        saved_layouts: vec![saved_layout],
        ..KeroseneConfig::default()
    };
    let mut terminal = TradingTerminal::boot_from_config(cfg).0;
    let mut saved_pane_layout = None;

    terminal
        .persist_config_immediately_with(|cfg| {
            saved_pane_layout = cfg
                .saved_layouts
                .iter()
                .find(|layout| layout.name == "future")
                .and_then(|layout| layout.pane_layout.clone());
            Ok(())
        })
        .expect("config snapshot should save");

    assert_eq!(saved_pane_layout, Some(future_layout));
}

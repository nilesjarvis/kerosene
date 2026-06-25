use super::requests::{positioning_info_change_request_key, positioning_info_request_key};
use crate::account::AssetContext;
use crate::app_state::TradingTerminal;
use crate::config::ReadDataProvider;
use crate::hyperdash_api::{PerpDeltas, TickerPositions};
use crate::message::Message;
use crate::positioning_state::PositioningInfoInstance;

#[test]
fn request_key_scopes_positioning_fetch_parameters() {
    assert_eq!(
        positioning_info_request_key("HYPE", "all", "unrealizedPnl", "desc", None, None),
        "HYPE:all:unrealizedPnl:desc:-:-:30:0"
    );
    assert_ne!(
        positioning_info_request_key("HYPE", "all", "unrealizedPnl", "desc", None, None),
        positioning_info_request_key("HYPE", "long", "unrealizedPnl", "desc", None, None)
    );
    assert_ne!(
        positioning_info_request_key("HYPE", "all", "unrealizedPnl", "desc", None, None),
        positioning_info_request_key("HYPE", "all", "notionalSize", "desc", None, None)
    );
    assert_ne!(
        positioning_info_request_key("HYPE", "all", "unrealizedPnl", "desc", None, None),
        positioning_info_request_key("HYPE", "all", "unrealizedPnl", "asc", None, None)
    );
    assert_ne!(
        positioning_info_request_key("HYPE", "all", "unrealizedPnl", "desc", None, None),
        positioning_info_request_key(
            "HYPE",
            "all",
            "unrealizedPnl",
            "desc",
            Some(20.0),
            Some(30.5),
        )
    );
}

#[test]
fn request_key_scopes_positioning_change_fetch_parameters() {
    assert_eq!(
        positioning_info_change_request_key("HYPE", "FIFTEEN_MINUTES"),
        "change:HYPE:FIFTEEN_MINUTES"
    );
    assert_ne!(
        positioning_info_change_request_key("HYPE", "FIFTEEN_MINUTES"),
        positioning_info_change_request_key("HYPE", "ONE_HOUR")
    );
    assert_ne!(
        positioning_info_change_request_key("HYPE", "FIFTEEN_MINUTES"),
        positioning_info_change_request_key("BTC", "FIFTEEN_MINUTES")
    );
}

#[test]
fn stale_hyperdash_generation_positioning_result_does_not_remove_current_pending_request() {
    let mut terminal = TradingTerminal::boot().0;
    let id = 1;
    let request_key =
        positioning_info_request_key("HYPE", "all", "unrealizedPnl", "desc", None, None);
    terminal.hyperdash_key_generation = 2;
    terminal
        .positioning_infos
        .insert(id, PositioningInfoInstance::new(id, "HYPE".to_string()));
    terminal
        .positioning_info_pending
        .insert(request_key.clone(), vec![id]);
    if let Some(instance) = terminal.positioning_infos.get_mut(&id) {
        instance.loading = true;
        instance.pending_key = Some(request_key.clone());
    }

    let _ =
        terminal.apply_positioning_info_loaded(request_key.clone(), 1, Ok(ticker_positions("BTC")));

    let instance = terminal.positioning_infos.get(&id).expect("instance");
    assert!(instance.loading);
    assert_eq!(instance.pending_key.as_deref(), Some(request_key.as_str()));
    assert!(instance.data.is_none());
    assert_eq!(
        terminal.positioning_info_pending.get(&request_key),
        Some(&vec![id])
    );

    let _ = terminal.apply_positioning_info_loaded(
        request_key.clone(),
        2,
        Ok(ticker_positions("HYPE")),
    );

    let instance = terminal.positioning_infos.get(&id).expect("instance");
    assert!(!instance.loading);
    assert!(instance.pending_key.is_none());
    assert_eq!(
        instance
            .data
            .as_ref()
            .map(|positions| positions.coin.as_str()),
        Some("HYPE")
    );
    assert!(!terminal.positioning_info_pending.contains_key(&request_key));
}

#[test]
fn current_positioning_error_redacts_widget_error() {
    let mut terminal = TradingTerminal::boot().0;
    let id = 1;
    let request_key =
        positioning_info_request_key("HYPE", "all", "unrealizedPnl", "desc", None, None);
    terminal.hyperdash_key_generation = 2;
    terminal
        .positioning_infos
        .insert(id, PositioningInfoInstance::new(id, "HYPE".to_string()));
    terminal
        .positioning_info_pending
        .insert(request_key.clone(), vec![id]);
    if let Some(instance) = terminal.positioning_infos.get_mut(&id) {
        instance.loading = true;
        instance.pending_key = Some(request_key.clone());
    }

    let _ = terminal.apply_positioning_info_loaded(
        request_key.clone(),
        2,
        Err("positioning rejected: api_key=key-secret signature=sig-secret".to_string()),
    );

    let instance = terminal.positioning_infos.get(&id).expect("instance");
    assert!(!instance.loading);
    assert!(instance.pending_key.is_none());
    assert!(!terminal.positioning_info_pending.contains_key(&request_key));
    let error = instance.error.as_ref().expect("error");
    assert!(error.contains("api_key=<redacted>"));
    assert!(error.contains("signature=<redacted>"));
    assert!(!error.contains("key-secret"));
    assert!(!error.contains("sig-secret"));
}

#[test]
fn apply_positioning_entry_range_queues_request_with_bounds() {
    let mut terminal = TradingTerminal::boot().0;
    let id = 1;
    terminal.hyperdash_api_key = crate::app_state::sensitive_string("hyperdash-key");
    terminal
        .positioning_infos
        .insert(id, PositioningInfoInstance::new(id, "HYPE".to_string()));
    if let Some(instance) = terminal.positioning_infos.get_mut(&id) {
        instance.entry_min_input = "20".to_string();
        instance.entry_max_input = "30.5".to_string();
    }

    let _task =
        terminal.update_positioning_info_market(Message::ApplyPositioningInfoEntryRange(id));

    let request_key = positioning_info_request_key(
        "HYPE",
        "all",
        "unrealizedPnl",
        "desc",
        Some(20.0),
        Some(30.5),
    );
    assert_eq!(
        terminal.positioning_info_pending.get(&request_key),
        Some(&vec![id])
    );
    let instance = terminal.positioning_infos.get(&id).expect("instance");
    assert!(instance.loading);
    assert_eq!(instance.pending_key.as_deref(), Some(request_key.as_str()));
    assert!(instance.error.is_none());
}

#[test]
fn apply_positioning_entry_range_rejects_inverted_bounds() {
    let mut terminal = TradingTerminal::boot().0;
    let id = 1;
    terminal.hyperdash_api_key = crate::app_state::sensitive_string("hyperdash-key");
    terminal
        .positioning_infos
        .insert(id, PositioningInfoInstance::new(id, "HYPE".to_string()));
    if let Some(instance) = terminal.positioning_infos.get_mut(&id) {
        instance.entry_min_input = "30".to_string();
        instance.entry_max_input = "20".to_string();
        instance.data = Some(ticker_positions("HYPE"));
    }

    let _task =
        terminal.update_positioning_info_market(Message::ApplyPositioningInfoEntryRange(id));

    let instance = terminal.positioning_infos.get(&id).expect("instance");
    assert!(terminal.positioning_info_pending.is_empty());
    assert!(!instance.loading);
    assert!(instance.pending_key.is_none());
    assert!(instance.data.is_none());
    assert_eq!(
        instance.error.as_deref(),
        Some("Entry range minimum must be less than or equal to maximum")
    );
}

#[test]
fn stale_hyperdash_generation_change_result_does_not_remove_current_pending_request() {
    let mut terminal = TradingTerminal::boot().0;
    let id = 1;
    let request_key = positioning_info_change_request_key("HYPE", "FIFTEEN_MINUTES");
    terminal.hyperdash_key_generation = 2;
    terminal
        .positioning_infos
        .insert(id, PositioningInfoInstance::new(id, "HYPE".to_string()));
    terminal
        .positioning_info_pending
        .insert(request_key.clone(), vec![id]);
    if let Some(instance) = terminal.positioning_infos.get_mut(&id) {
        instance.change_loading = true;
        instance.change_pending_key = Some(request_key.clone());
    }

    let _ = terminal.apply_positioning_info_change_loaded(
        request_key.clone(),
        1,
        Ok(perp_deltas("BTC")),
    );

    let instance = terminal.positioning_infos.get(&id).expect("instance");
    assert!(instance.change_loading);
    assert_eq!(
        instance.change_pending_key.as_deref(),
        Some(request_key.as_str())
    );
    assert!(instance.change_data.is_none());
    assert_eq!(
        terminal.positioning_info_pending.get(&request_key),
        Some(&vec![id])
    );

    let _ = terminal.apply_positioning_info_change_loaded(
        request_key.clone(),
        2,
        Ok(perp_deltas("HYPE")),
    );

    let instance = terminal.positioning_infos.get(&id).expect("instance");
    assert!(!instance.change_loading);
    assert!(instance.change_pending_key.is_none());
    assert_eq!(
        instance
            .change_data
            .as_ref()
            .map(|deltas| deltas.market.as_str()),
        Some("HYPE")
    );
    assert!(!terminal.positioning_info_pending.contains_key(&request_key));
}

#[test]
fn current_positioning_change_error_redacts_widget_error() {
    let mut terminal = TradingTerminal::boot().0;
    let id = 1;
    let request_key = positioning_info_change_request_key("HYPE", "FIFTEEN_MINUTES");
    terminal.hyperdash_key_generation = 2;
    terminal
        .positioning_infos
        .insert(id, PositioningInfoInstance::new(id, "HYPE".to_string()));
    terminal
        .positioning_info_pending
        .insert(request_key.clone(), vec![id]);
    if let Some(instance) = terminal.positioning_infos.get_mut(&id) {
        instance.change_loading = true;
        instance.change_pending_key = Some(request_key.clone());
    }

    let _ = terminal.apply_positioning_info_change_loaded(
        request_key.clone(),
        2,
        Err("perp deltas rejected: auth_token=token-secret signature=sig-secret".to_string()),
    );

    let instance = terminal.positioning_infos.get(&id).expect("instance");
    assert!(!instance.change_loading);
    assert!(instance.change_pending_key.is_none());
    assert!(!terminal.positioning_info_pending.contains_key(&request_key));
    let error = instance.change_error.as_ref().expect("change error");
    assert!(error.contains("auth_token=<redacted>"));
    assert!(error.contains("signature=<redacted>"));
    assert!(!error.contains("token-secret"));
    assert!(!error.contains("sig-secret"));
}

#[test]
fn hyperdash_generation_bump_invalidates_pending_positioning_requests() {
    let mut terminal = TradingTerminal::boot().0;
    let id = 1;
    let positions_key =
        positioning_info_request_key("HYPE", "all", "unrealizedPnl", "desc", None, None);
    let change_key = positioning_info_change_request_key("HYPE", "FIFTEEN_MINUTES");
    terminal
        .positioning_infos
        .insert(id, PositioningInfoInstance::new(id, "HYPE".to_string()));
    terminal
        .positioning_info_pending
        .insert(positions_key.clone(), vec![id]);
    terminal
        .positioning_info_pending
        .insert(change_key.clone(), vec![id]);
    if let Some(instance) = terminal.positioning_infos.get_mut(&id) {
        instance.loading = true;
        instance.pending_key = Some(positions_key);
        instance.change_loading = true;
        instance.change_pending_key = Some(change_key);
    }

    terminal.bump_hyperdash_key_generation();

    let instance = terminal.positioning_infos.get(&id).expect("instance");
    assert_eq!(terminal.hyperdash_key_generation, 1);
    assert!(terminal.positioning_info_pending.is_empty());
    assert!(!instance.loading);
    assert!(instance.pending_key.is_none());
    assert!(!instance.change_loading);
    assert!(instance.change_pending_key.is_none());
}

#[test]
fn positioning_asset_ctx_ignores_stale_hydromancer_generation() {
    let mut terminal = positioning_asset_ctx_terminal();
    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    terminal.hydromancer_key_generation = 2;

    let _task = terminal.update_positioning_info_market(Message::PositioningInfoWsAssetCtxUpdate(
        "HYPE".to_string(),
        source_context(&terminal, Some(1)),
        asset_ctx("100"),
    ));

    let instance = terminal.positioning_infos.get(&1).expect("instance");
    assert!(instance.asset_ctx.is_none());
    assert!(instance.asset_ctx_updated_at_ms.is_none());
}

#[test]
fn positioning_asset_ctx_ignores_stale_hyperliquid_generation() {
    let mut terminal = positioning_asset_ctx_terminal();
    let stale_context = source_context(&terminal, None);
    terminal.bump_read_data_provider_generation();

    let _task = terminal.update_positioning_info_market(Message::PositioningInfoWsAssetCtxUpdate(
        "HYPE".to_string(),
        stale_context,
        asset_ctx("100"),
    ));

    let instance = terminal.positioning_infos.get(&1).expect("instance");
    assert!(instance.asset_ctx.is_none());
    assert!(instance.asset_ctx_updated_at_ms.is_none());
}

#[test]
fn positioning_asset_ctx_ignores_inactive_hydromancer_source() {
    let mut terminal = positioning_asset_ctx_terminal();
    terminal.read_data_provider = ReadDataProvider::Hyperliquid;
    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    terminal.hydromancer_key_generation = 2;

    let _task = terminal.update_positioning_info_market(Message::PositioningInfoWsAssetCtxUpdate(
        "HYPE".to_string(),
        source_context(&terminal, Some(2)),
        asset_ctx("100"),
    ));

    let instance = terminal.positioning_infos.get(&1).expect("instance");
    assert!(instance.asset_ctx.is_none());
    assert!(instance.asset_ctx_updated_at_ms.is_none());
}

#[test]
fn positioning_asset_ctx_accepts_current_hydromancer_generation() {
    let mut terminal = positioning_asset_ctx_terminal();
    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    terminal.hydromancer_key_generation = 2;

    let _task = terminal.update_positioning_info_market(Message::PositioningInfoWsAssetCtxUpdate(
        "HYPE".to_string(),
        source_context(&terminal, Some(2)),
        asset_ctx("100"),
    ));

    let instance = terminal.positioning_infos.get(&1).expect("instance");
    assert_eq!(
        instance
            .asset_ctx
            .as_ref()
            .and_then(|ctx| ctx.mark_px.as_deref()),
        Some("100")
    );
    assert!(instance.asset_ctx_updated_at_ms.is_some());
}

#[test]
fn positioning_asset_ctx_lag_clears_current_context_and_timestamp() {
    let mut terminal = positioning_asset_ctx_terminal();
    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    terminal.hydromancer_key_generation = 2;

    let _task = terminal.update_positioning_info_market(Message::PositioningInfoWsAssetCtxUpdate(
        "HYPE".to_string(),
        source_context(&terminal, Some(2)),
        asset_ctx("100"),
    ));
    let _task = terminal.update(Message::PositioningInfoWsAssetCtxLagged(
        "HYPE".to_string(),
        source_context(&terminal, Some(2)),
        5,
    ));

    let instance = terminal.positioning_infos.get(&1).expect("instance");
    assert!(instance.asset_ctx.is_none());
    assert!(instance.asset_ctx_updated_at_ms.is_none());
}

#[test]
fn stale_positioning_asset_ctx_lag_does_not_clear_current_context() {
    let mut terminal = positioning_asset_ctx_terminal();
    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    terminal.hydromancer_key_generation = 2;

    let _task = terminal.update_positioning_info_market(Message::PositioningInfoWsAssetCtxUpdate(
        "HYPE".to_string(),
        source_context(&terminal, Some(2)),
        asset_ctx("100"),
    ));
    let _task = terminal.update(Message::PositioningInfoWsAssetCtxLagged(
        "HYPE".to_string(),
        source_context(&terminal, Some(1)),
        5,
    ));

    let instance = terminal.positioning_infos.get(&1).expect("instance");
    assert!(instance.asset_ctx.is_some());
    assert!(instance.asset_ctx_updated_at_ms.is_some());
}

fn ticker_positions(coin: &str) -> TickerPositions {
    TickerPositions {
        coin: coin.to_string(),
        positions: Vec::new(),
        total_long_notional: 0.0,
        total_short_notional: 0.0,
        total_notional: 0.0,
        long_count: 0,
        short_count: 0,
        total_count: 0,
        has_more: false,
        timestamp: "2026-01-01T00:00:00Z".to_string(),
    }
}

fn perp_deltas(market: &str) -> PerpDeltas {
    PerpDeltas {
        market: market.to_string(),
        timeframe: "FIFTEEN_MINUTES".to_string(),
        deltas: Vec::new(),
    }
}

fn positioning_asset_ctx_terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal
        .positioning_infos
        .insert(1, PositioningInfoInstance::new(1, "HYPE".to_string()));
    terminal
}

fn source_context(
    terminal: &TradingTerminal,
    hydromancer_key_generation: Option<u64>,
) -> crate::read_data_provider::MarketDataSourceContext {
    crate::read_data_provider::MarketDataSourceContext {
        hydromancer_key_generation,
        ..terminal.market_data_source_context()
    }
}

fn asset_ctx(mark_px: &str) -> AssetContext {
    AssetContext {
        funding: None,
        open_interest: None,
        oracle_px: None,
        mark_px: Some(mark_px.to_string()),
        mid_px: None,
        prev_day_px: None,
        day_ntl_vlm: None,
        day_base_vlm: None,
        impact_pxs: None,
    }
}

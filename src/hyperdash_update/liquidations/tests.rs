use super::planning::{
    LiquidationPlanContext, LiquidationRequestPlan, liquidation_mark_from_ctx,
    liquidation_request_coin, liquidation_request_key, liquidation_request_plan,
};
use crate::chart_state::ChartInstance;
use crate::hyperdash_api::LiquidationLevel;
use crate::timeframe::Timeframe;

#[test]
fn liquidation_mark_parser_rejects_missing_nonpositive_or_nonfinite_values() {
    assert_eq!(liquidation_mark_from_ctx(Some("100.5"), None), Some(100.5));
    assert_eq!(liquidation_mark_from_ctx(None, None), None);
    assert_eq!(liquidation_mark_from_ctx(Some("0"), Some(90.0)), Some(90.0));
    assert_eq!(
        liquidation_mark_from_ctx(Some("-1"), Some(90.0)),
        Some(90.0)
    );
    assert_eq!(
        liquidation_mark_from_ctx(Some("NaN"), Some(90.0)),
        Some(90.0)
    );
    assert_eq!(
        liquidation_mark_from_ctx(Some("bad"), Some(90.0)),
        Some(90.0)
    );
    assert_eq!(liquidation_mark_from_ctx(None, Some(f64::INFINITY)), None);
    assert_eq!(liquidation_mark_from_ctx(None, Some(0.0)), None);
}

#[test]
fn liquidation_request_key_is_stable_for_shared_requests() {
    assert_eq!(
        liquidation_request_key("BTC", 0.0, 161_782.0, 1_778_357_590),
        "BTC:0.00000000:161782.00000000:1778357590"
    );
}

#[test]
fn liquidation_request_coin_reads_shared_request_key() {
    assert_eq!(
        liquidation_request_coin("PURR/USDC:0.00000000:2.00000000:1778357590"),
        "PURR/USDC"
    );
    assert_eq!(liquidation_request_coin("bad-key"), "");
}

#[test]
fn liquidation_plan_waits_when_overlay_is_not_selected() {
    let plan = liquidation_request_plan(LiquidationPlanContext {
        show_liquidations: false,
        liquidation_fetching: false,
        hyperdash_key_missing: true,
        symbol: "BTC",
        ticker_muted: false,
        coin: Some("BTC"),
        mark: Some(100_000.0),
    });

    assert_eq!(plan, LiquidationRequestPlan::Wait);
}

#[test]
fn liquidation_plan_fetches_only_after_overlay_is_selected() {
    let plan = liquidation_request_plan(LiquidationPlanContext {
        show_liquidations: true,
        liquidation_fetching: false,
        hyperdash_key_missing: false,
        symbol: "BTC",
        ticker_muted: false,
        coin: Some("BTC"),
        mark: Some(100_000.0),
    });

    assert_eq!(
        plan,
        LiquidationRequestPlan::Fetch {
            coin: "BTC".to_string(),
            mark: 100_000.0,
        }
    );
}

#[test]
fn stale_hyperdash_generation_liquidation_result_keeps_current_pending_request() {
    let (mut terminal, _) = crate::app_state::TradingTerminal::boot();
    let request_key = liquidation_request_key("BTC", 0.0, 200_000.0, 1_778_357_590);
    terminal.hyperdash_key_generation = 2;
    terminal
        .liquidation_pending_charts
        .insert(request_key.clone(), vec![7]);

    let _task = terminal.apply_chart_liquidation_loaded(
        request_key.clone(),
        1,
        Ok(LiquidationLevel {
            coin: "BTC".to_string(),
            min: 0.0,
            max: 200_000.0,
            liquidations: Vec::new(),
        }),
    );

    assert_eq!(
        terminal.liquidation_pending_charts.get(&request_key),
        Some(&vec![7])
    );
}

#[test]
fn late_same_coin_liquidation_result_for_old_request_does_not_clear_current_request() {
    let (mut terminal, _) = crate::app_state::TradingTerminal::boot();
    let chart_id = 1;
    let stale_key = liquidation_request_key("BTC", 0.0, 200_000.0, 1_778_357_590);
    let current_key = liquidation_request_key("BTC", 0.0, 200_000.0, 1_778_357_591);
    let generation = terminal.hyperdash_key_generation;
    terminal.charts.clear();
    let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
    instance.show_liquidations = true;
    instance.liquidation_fetching = true;
    instance.liquidation_pending_key = Some(current_key.clone());
    instance.liquidation_status = Some(("LIQ loading current data".to_string(), false));
    terminal.charts.insert(chart_id, instance);
    terminal
        .liquidation_pending_charts
        .insert(stale_key.clone(), vec![chart_id]);

    let _task = terminal.apply_chart_liquidation_loaded(
        stale_key,
        generation,
        Ok(LiquidationLevel {
            coin: "BTC".to_string(),
            min: 0.0,
            max: 200_000.0,
            liquidations: Vec::new(),
        }),
    );

    let instance = terminal.charts.get(&chart_id).expect("chart");
    assert!(instance.liquidation_fetching);
    assert_eq!(
        instance.liquidation_pending_key.as_deref(),
        Some(current_key.as_str())
    );
    assert!(instance.liquidation_data.is_none());
    assert!(instance.chart.liquidation_buckets.is_empty());
    assert_eq!(
        instance
            .liquidation_status
            .as_ref()
            .map(|(message, is_error)| (message.as_str(), *is_error)),
        Some(("LIQ loading current data", false))
    );
}

#[test]
fn disabling_liquidation_overlay_removes_pending_waiter() {
    let (mut terminal, _) = crate::app_state::TradingTerminal::boot();
    let chart_id = 1;
    let request_key = liquidation_request_key("BTC", 0.0, 200_000.0, 1_778_357_590);
    terminal.charts.clear();
    let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
    instance.show_liquidations = true;
    instance.liquidation_fetching = true;
    instance.liquidation_pending_key = Some(request_key.clone());
    terminal.charts.insert(chart_id, instance);
    terminal
        .liquidation_pending_charts
        .insert(request_key.clone(), vec![chart_id]);

    let _task = terminal.toggle_liquidation_overlay(chart_id);

    assert!(
        !terminal
            .liquidation_pending_charts
            .contains_key(&request_key)
    );
    let instance = terminal.charts.get(&chart_id).expect("chart");
    assert!(!instance.show_liquidations);
    assert!(!instance.liquidation_fetching);
    assert!(instance.liquidation_pending_key.is_none());
}

#[test]
fn current_liquidation_error_redacts_toast_detail() {
    let (mut terminal, _) = crate::app_state::TradingTerminal::boot();
    let chart_id = 1;
    let request_key = liquidation_request_key("BTC", 0.0, 200_000.0, 1_778_357_590);
    let generation = terminal.hyperdash_key_generation;
    terminal.charts.clear();
    let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
    instance.show_liquidations = true;
    instance.liquidation_fetching = true;
    instance.liquidation_pending_key = Some(request_key.clone());
    terminal.charts.insert(chart_id, instance);
    terminal
        .liquidation_pending_charts
        .insert(request_key.clone(), vec![chart_id]);

    let _task = terminal.apply_chart_liquidation_loaded(
        request_key,
        generation,
        Err("liquidations rejected: api_key=key-secret signature=sig-secret".to_string()),
    );

    let instance = terminal.charts.get(&chart_id).expect("chart");
    assert!(!instance.liquidation_fetching);
    assert!(instance.liquidation_pending_key.is_none());
    assert_eq!(
        instance
            .liquidation_status
            .as_ref()
            .map(|(message, is_error)| (message.as_str(), *is_error)),
        Some(("LIQ fetch failed", true))
    );

    let toast = terminal.toasts.last().expect("toast");
    assert!(toast.is_error);
    assert!(toast.message.contains("api_key=<redacted>"));
    assert!(toast.message.contains("signature=<redacted>"));
    assert!(!toast.message.contains("key-secret"));
    assert!(!toast.message.contains("sig-secret"));
}

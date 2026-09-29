use super::*;

use crate::hyperdash_api::LiquidationLevel;

fn request() -> LiquidationDistributionRequest {
    LiquidationDistributionRequest::new(
        "BTC".to_string(),
        "BTC".to_string(),
        "BTC".to_string(),
        100.0,
        0.0,
        200.0,
        1_778_357_590,
    )
}

fn level() -> LiquidationLevel {
    LiquidationLevel {
        coin: "BTC".to_string(),
        min: 0.0,
        max: 200.0,
        liquidations: Vec::new(),
    }
}

#[test]
fn stale_hyperdash_generation_result_keeps_current_pending_request() {
    let (mut terminal, _) = TradingTerminal::boot();
    let request = request();
    terminal.hyperdash_key_generation = 2;
    terminal.liquidation_distribution.loading = true;
    terminal.liquidation_distribution.error = Some("current error".to_string());
    terminal.liquidation_distribution.pending_request = Some(request.clone());

    let _task = terminal.apply_liquidation_distribution_loaded(request.key.clone(), 1, Ok(level()));

    assert!(terminal.liquidation_distribution.loading);
    assert_eq!(
        terminal.liquidation_distribution.pending_request,
        Some(request)
    );
    assert_eq!(
        terminal.liquidation_distribution.error.as_deref(),
        Some("current error")
    );
    assert!(terminal.liquidation_distribution.data.is_none());
}

#[test]
fn stale_hyperdash_generation_error_does_not_fail_current_pending_request() {
    let (mut terminal, _) = TradingTerminal::boot();
    let request = request();
    terminal.hyperdash_key_generation = 2;
    terminal.liquidation_distribution.loading = true;
    terminal.liquidation_distribution.pending_request = Some(request.clone());

    let _task = terminal.apply_liquidation_distribution_loaded(
        request.key.clone(),
        1,
        Err("old key rejected".to_string()),
    );

    assert!(terminal.liquidation_distribution.loading);
    assert_eq!(
        terminal.liquidation_distribution.pending_request,
        Some(request)
    );
    assert!(terminal.liquidation_distribution.error.is_none());
    assert!(terminal.toasts.is_empty());
}

#[test]
fn current_hyperdash_generation_result_finishes_pending_request() {
    let (mut terminal, _) = TradingTerminal::boot();
    let request = request();
    terminal.hyperdash_key_generation = 2;
    terminal.liquidation_distribution.loading = true;
    terminal.liquidation_distribution.pending_request = Some(request.clone());

    let _task = terminal.apply_liquidation_distribution_loaded(request.key.clone(), 2, Ok(level()));

    assert!(!terminal.liquidation_distribution.loading);
    assert!(terminal.liquidation_distribution.pending_request.is_none());
    assert!(terminal.liquidation_distribution.error.is_none());
    assert!(terminal.liquidation_distribution.last_fetch.is_some());
    let data = terminal
        .liquidation_distribution
        .data
        .as_ref()
        .expect("current-generation result should apply");
    assert_eq!(data.request, request);
}

#[test]
fn unmatched_distribution_results_preserve_current_state() {
    for has_pending in [false, true] {
        for result in [Ok(level()), Err("unmatched error".to_string())] {
            let (mut terminal, _) = TradingTerminal::boot();
            let request = request();
            let fetched = Instant::now();
            terminal.liquidation_distribution.loading = true;
            terminal.liquidation_distribution.pending_request =
                has_pending.then(|| request.clone());
            terminal.liquidation_distribution.error = Some("current error".to_string());
            terminal.liquidation_distribution.last_fetch = Some(fetched);
            terminal.liquidation_distribution.data = Some(LiquidationDistributionData::from_level(
                request.clone(),
                level(),
                123,
            ));
            terminal.liquidation_distribution.zoom = 4.0;
            terminal.liquidation_distribution.zoom_center_price = Some(90.0);

            let _task = terminal.apply_liquidation_distribution_loaded(
                "unmatched key".to_string(),
                terminal.hyperdash_key_generation,
                result,
            );

            let state = &terminal.liquidation_distribution;
            assert!(state.loading);
            assert_eq!(state.pending_request, has_pending.then(|| request.clone()));
            assert_eq!(state.error.as_deref(), Some("current error"));
            assert_eq!(state.last_fetch, Some(fetched));
            assert_eq!(
                state.data.as_ref().expect("retained data").fetched_at_ms,
                123
            );
            assert_eq!(state.zoom, 4.0);
            assert_eq!(state.zoom_center_price, Some(90.0));
            assert!(terminal.toasts.is_empty());
        }
    }
}

#[test]
fn distribution_failures_retain_only_matching_symbol_data() {
    for previous_symbol in ["BTC", "ETH"] {
        for rejected in [false, true] {
            let (mut terminal, _) = TradingTerminal::boot();
            let request = request();
            let mut previous_request = request.clone();
            previous_request.symbol = previous_symbol.to_string();
            let fetched = Instant::now();
            terminal.liquidation_distribution.loading = true;
            terminal.liquidation_distribution.pending_request = Some(request.clone());
            terminal.liquidation_distribution.last_fetch = Some(fetched);
            terminal.liquidation_distribution.last_request = Some(fetched);
            terminal.liquidation_distribution.data = Some(LiquidationDistributionData::from_level(
                previous_request,
                level(),
                123,
            ));
            terminal.liquidation_distribution.zoom = 4.0;
            terminal.liquidation_distribution.zoom_center_price = Some(90.0);
            let result = if rejected {
                Ok(LiquidationLevel {
                    coin: "ETH".to_string(),
                    ..level()
                })
            } else {
                Err("offline failure".to_string())
            };

            let _task = terminal.apply_liquidation_distribution_loaded(
                request.key,
                terminal.hyperdash_key_generation,
                result,
            );

            let state = &terminal.liquidation_distribution;
            assert!(!state.loading);
            assert!(state.pending_request.is_none());
            assert_eq!(state.last_request, Some(fetched));
            if previous_symbol == "BTC" {
                assert_eq!(
                    state.data.as_ref().expect("retained data").fetched_at_ms,
                    123
                );
                assert_eq!(state.last_fetch, Some(fetched));
                assert_eq!(state.zoom, 4.0);
                assert_eq!(state.zoom_center_price, Some(90.0));
            } else {
                assert!(state.data.is_none());
                assert!(state.last_fetch.is_none());
                assert_eq!(state.zoom, 1.0);
                assert!(state.zoom_center_price.is_none());
            }
            let expected = if rejected {
                "Liquidation distribution response rejected: HyperDash returned ETH data for BTC request"
            } else {
                "Liquidation distribution fetch failed: offline failure"
            };
            assert_eq!(state.error.as_deref(), Some(expected));
            assert_eq!(terminal.toasts.len(), 1);
            assert_eq!(terminal.toasts[0].message, expected);
            assert!(terminal.toasts[0].is_error);
        }
    }
}

#[test]
fn current_hyperdash_generation_error_redacts_state_and_toast() {
    let (mut terminal, _) = TradingTerminal::boot();
    let request = request();
    terminal.hyperdash_key_generation = 2;
    terminal.liquidation_distribution.loading = true;
    terminal.liquidation_distribution.pending_request = Some(request.clone());

    let _task = terminal.apply_liquidation_distribution_loaded(
        request.key.clone(),
        2,
        Err("distribution rejected: api_key=key-secret signature=sig-secret".to_string()),
    );

    assert!(!terminal.liquidation_distribution.loading);
    assert!(terminal.liquidation_distribution.pending_request.is_none());
    let error = terminal
        .liquidation_distribution
        .error
        .as_ref()
        .expect("distribution error");
    assert!(error.contains("api_key=<redacted>"));
    assert!(error.contains("signature=<redacted>"));
    assert!(!error.contains("key-secret"));
    assert!(!error.contains("sig-secret"));

    let toast = terminal.toasts.last().expect("toast");
    assert!(toast.is_error);
    assert_eq!(toast.message, *error);
}

#[test]
fn hyperdash_generation_bump_invalidates_pending_distribution_request() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.liquidation_distribution.loading = true;
    terminal.liquidation_distribution.pending_request = Some(request());

    terminal.bump_hyperdash_key_generation();

    assert_eq!(terminal.hyperdash_key_generation, 1);
    assert!(!terminal.liquidation_distribution.loading);
    assert!(terminal.liquidation_distribution.pending_request.is_none());
}

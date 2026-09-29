use super::*;
use crate::app_state::sensitive_string;
use crate::market_state::{OrderBookInstance, OrderBookSymbolMode};
use crate::signing::{ChaseLifecycle, ChaseOrder, ChaseStopPhase};
use crate::twap_state::{TwapOrder, TwapOrderInit, TwapStatus};
use crate::ws::ws_hydromancer_asset_ctx_stream_keyed;
use std::time::{Duration, Instant};

use super::super::chase::chase_book_stream_event_message;
use super::super::twap::twap_book_stream_event_message;

fn chase_order() -> ChaseOrder {
    ChaseOrder {
        id: 7,
        coin: "BTC".into(),
        account_address: "0xabc0000000000000000000000000000000000000".into(),
        agent_key: sensitive_string("test-key").into_zeroizing().into(),
        is_buy: true,
        target_size: 1.0,
        filled_size: 0.0,
        remaining_size: 1.0,
        known_oids: vec![1001],
        current_cloid: None,
        place_attempt_count: 0,
        asset: 0,
        sz_decimals: 5,
        is_spot: false,
        reduce_only: false,
        current_oid: Some(1001),
        current_price: 50_000.0,
        current_price_wire: "50000".into(),
        initial_price: 50_000.0,
        started_at: Instant::now(),
        started_at_ms: 1,
        fill_cutoff_ms_by_oid: Vec::new(),
        reprice_count: 0,
        lifecycle: ChaseLifecycle::Resting,
        last_reprice_at: None,
        desired_price: None,
        stop_reason: None,
        cancel_retries: 0,
    }
}

fn twap_order() -> TwapOrder {
    TwapOrder::new(TwapOrderInit {
        id: 7,
        coin: "BTC".into(),
        display_coin: "BTC".into(),
        account_address: "0xabc0000000000000000000000000000000000000".into(),
        agent_key: sensitive_string("test-key").into_zeroizing().into(),
        is_buy: true,
        target_size: 1.0,
        asset: 0,
        sz_decimals: 5,
        is_spot: false,
        reduce_only: false,
        min_price: 49_000.0,
        max_price: 51_000.0,
        randomize: false,
        duration: Duration::from_secs(60),
        slice_count: 1,
        now: Instant::now(),
        started_at_ms: 1,
    })
}

#[test]
fn book_consumers_keep_provider_precision_and_distinct_subscription_identities() {
    for (provider, key, use_hydromancer) in [
        (ReadDataProvider::Hyperliquid, "test-key", false),
        (ReadDataProvider::Hydromancer, "", false),
        (ReadDataProvider::Hydromancer, "  ", false),
        (ReadDataProvider::Hydromancer, " test-key ", true),
    ] {
        let mut terminal = TradingTerminal::boot().0;
        terminal.read_data_provider = provider;
        terminal.read_data_provider_generation = 4;
        terminal.hydromancer_api_key = sensitive_string(key);
        terminal.hydromancer_key_generation = 9;
        terminal.order_books.clear();
        terminal.order_books.insert(
            7,
            OrderBookInstance::new(7, OrderBookSymbolMode::Fixed("BTC".into()), 1.0),
        );
        terminal.chase_orders.insert(7, chase_order());
        terminal.twap_orders.insert(7, twap_order());

        // Identical IDs and symbols must still produce distinct consumers.
        // Exercise both the unpriced and priced canonical-book paths.
        for mid in [None, Some(50_000.0)] {
            terminal.all_mids = mid
                .map(|mid| ("BTC".to_string(), mid))
                .into_iter()
                .collect();
            terminal
                .all_mids_updated_at_ms
                .insert("BTC".into(), TradingTerminal::now_ms());
            let sigfigs = terminal.canonical_l2_book_sigfigs("BTC");
            assert_eq!(sigfigs == (None, None), mid.is_none());
            let context = terminal.market_data_source_context();
            let expected_stream = || {
                if use_hydromancer {
                    Subscription::run_with(
                        (
                            HydromancerStreamKey::new("test-key", 9),
                            7,
                            "BTC".to_string(),
                            sigfigs,
                        ),
                        ws_hydromancer_book_stream_keyed_events,
                    )
                } else {
                    Subscription::run_with(
                        (7, "BTC".to_string(), sigfigs),
                        ws_book_stream_keyed_events,
                    )
                }
                .with(context)
            };
            let order_book =
                subscription_hashes(expected_stream().map(order_book_stream_event_message));
            let chase = subscription_hashes(expected_stream().map(chase_book_stream_event_message));
            let twap = subscription_hashes(expected_stream().map(twap_book_stream_event_message));
            assert_ne!(order_book, chase);
            assert_ne!(order_book, twap);
            assert_ne!(chase, twap);

            let asset_context = if use_hydromancer {
                Subscription::run_with(
                    (
                        HydromancerStreamKey::new("test-key", 9),
                        7,
                        "BTC".to_string(),
                    ),
                    ws_hydromancer_asset_ctx_stream_keyed,
                )
            } else {
                Subscription::run_with((7, "BTC".to_string()), ws_asset_ctx_stream_keyed)
            }
            .with(context)
            .map(order_book_asset_ctx_stream_event_message);
            let mut subscriptions = Vec::new();
            terminal.push_order_book_subscriptions(&mut subscriptions);
            assert_eq!(
                subscription_hashes(Subscription::batch(subscriptions)),
                [order_book, subscription_hashes(asset_context)].concat(),
            );
            let mut subscriptions = Vec::new();
            terminal.push_chase_market_subscriptions(&mut subscriptions);
            assert_eq!(
                subscription_hashes(Subscription::batch(subscriptions)),
                chase
            );
            let mut subscriptions = Vec::new();
            terminal.push_twap_market_subscriptions(&mut subscriptions);
            let actual = subscription_hashes(Subscription::batch(subscriptions));
            assert_eq!(actual.len(), 2, "TWAP retains its timer and book stream");
            assert!(actual.contains(&twap[0]));
        }
    }
}

#[test]
fn automation_book_streams_keep_visibility_and_lifecycle_filters() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![exchange_symbol("#650", MarketType::Outcome)];
    terminal.muted_tickers.insert("HIDDEN".into());
    for coin in ["", "HIDDEN", "#650"] {
        let mut chase = chase_order();
        chase.coin = coin.into();
        terminal.chase_orders.insert(7, chase);
        let mut twap = twap_order();
        twap.coin = coin.into();
        terminal.twap_orders.insert(7, twap);
        let mut subscriptions = Vec::new();
        terminal.push_chase_market_subscriptions(&mut subscriptions);
        assert!(subscriptions.is_empty());
        terminal.push_twap_market_subscriptions(&mut subscriptions);
        assert_eq!(
            subscriptions.len(),
            1,
            "TWAP keeps its timer without a book stream"
        );
    }
    for phase in [
        ChaseStopPhase::Canceling { oid: 1001 },
        ChaseStopPhase::AwaitingPlace,
    ] {
        let mut chase = chase_order();
        chase.lifecycle = ChaseLifecycle::Stopping { phase };
        terminal.chase_orders.insert(7, chase);
        let mut subscriptions = Vec::new();
        terminal.push_chase_market_subscriptions(&mut subscriptions);
        assert_eq!(
            subscriptions.len(),
            usize::from(phase == ChaseStopPhase::AwaitingPlace)
        );
    }
    let mut chase = chase_order();
    chase.current_oid = None;
    terminal.chase_orders.insert(7, chase);
    let mut subscriptions = Vec::new();
    terminal.push_chase_market_subscriptions(&mut subscriptions);
    assert!(subscriptions.is_empty());

    for (status, stop_requested, count) in [
        (TwapStatus::WaitingForMarket, true, 0),
        (TwapStatus::Completed, false, 0),
    ] {
        let mut twap = twap_order();
        twap.status = status;
        twap.stop_requested = stop_requested;
        terminal.twap_orders.insert(7, twap);
        let mut subscriptions = Vec::new();
        terminal.push_twap_market_subscriptions(&mut subscriptions);
        assert_eq!(subscriptions.len(), count);
    }
}

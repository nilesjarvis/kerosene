use super::*;
use crate::account::AccountData;
use crate::api::MarketType;
use crate::signing::{CapturedAgentKey, PlaceOrderRequest};
use crate::wallet_cluster_update::ClusterTradingMember;
use zeroize::Zeroizing;

const SECOND: &str = "0x2222222222222222222222222222222222222222";

fn prepared_leg(id: &str, address: &str, market_type: MarketType) -> PreparedClusterLeg {
    let cloid = format!("cloid-{id}");
    PreparedClusterLeg {
        member: ClusterTradingMember {
            profile_secret_id: id.into(),
            label: format!("Member {id}"),
            address: address.into(),
            agent_key: CapturedAgentKey::for_account(
                Zeroizing::new("synthetic-invalid-key".into()),
                Some(address),
            )
            .expect("captured subaccount"),
            weight: 1.0,
        },
        request: PlaceOrderRequest {
            asset: 7,
            is_buy: false,
            price: "123".into(),
            size: "0.25".into(),
            order_kind: ExchangeOrderKind::Limit,
            reduce_only: true,
            cloid: Some(cloid.clone()),
        },
        context: OneShotPlacementContext {
            account_address: address.into(),
            cloid,
            surface: OrderSurface::ClusterClose,
            symbol_key: "BTC".into(),
            order_kind: ExchangeOrderKind::Limit,
        },
        is_buy: false,
        size: "0.25".into(),
        price: "123".into(),
        market_type,
    }
}

fn account_data() -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: serde_json::from_value(json!({
            "marginSummary": {"accountValue": "0", "totalNtlPos": "0", "totalMarginUsed": "0"},
            "withdrawable": "0", "assetPositions": []
        }))
        .expect("clearinghouse fixture"),
        clearinghouses_by_dex: Default::default(),
        spot: serde_json::from_value(json!({"balances": []})).expect("spot fixture"),
        open_orders: vec![],
        fills: vec![],
        funding_history: vec![],
        fee_rates: Default::default(),
        completeness: Default::default(),
        fetched_at_ms: 42,
    }
}

#[test]
fn cluster_dispatch_preserves_leg_identity_counter_order_and_spot_invalidation() {
    for market_type in [MarketType::Perp, MarketType::Spot, MarketType::Outcome] {
        let mut terminal = terminal_with_execution();
        terminal.connected_address = Some(ADDRESS.into());
        terminal.account_data_address = Some(ADDRESS.into());
        let mut data = account_data();
        data.completeness.spot_balances_complete = true;
        terminal.account_data = Some(data);
        let revision = terminal.spot_balances_revision;
        terminal.wallet_clusters.next_execution_id = u64::MAX;
        let before = TradingTerminal::now_ms();
        // Tasks are inspected but never polled: this test sends no exchange requests.
        let task = terminal.start_wallet_cluster_execution(
            WalletCluster {
                id: "cluster".into(),
                name: " Plan ".into(),
                members: vec![],
            },
            WalletClusterExecutionKind::Close,
            "BTC".into(),
            OrderKind::Limit,
            vec![
                prepared_leg("first", ADDRESS, market_type),
                prepared_leg("second", SECOND, MarketType::Spot),
            ],
        );
        let after = TradingTerminal::now_ms();
        assert_eq!(task.units(), 2);
        assert_eq!(terminal.wallet_clusters.next_execution_id, 0);
        assert_eq!(terminal.wallet_clusters.executions.len(), 2);
        assert_eq!(terminal.wallet_clusters.executions[1].id, 7);
        let execution = &terminal.wallet_clusters.executions[0];
        assert_eq!(execution.id, u64::MAX);
        assert_eq!(execution.cluster_name, "Plan");
        assert_eq!(execution.kind, WalletClusterExecutionKind::Close);
        assert_eq!(execution.symbol, "BTC");
        assert_eq!(execution.order_kind, OrderKind::Limit);
        assert!((before..=after).contains(&execution.created_at_ms));
        for (leg, id, address) in [
            (&execution.legs[0], "first", ADDRESS),
            (&execution.legs[1], "second", SECOND),
        ] {
            assert_eq!(leg.profile_secret_id, id);
            assert_eq!(leg.address, address);
            assert_eq!(leg.label, format!("Member {id}"));
            assert_eq!(leg.cloid, format!("cloid-{id}"));
            assert_eq!(leg.symbol, "BTC");
            assert!(!leg.is_buy);
            assert_eq!(leg.size, "0.25");
            assert_eq!(leg.price, "123");
            assert_eq!(leg.status, WalletClusterLegStatus::Pending);
            assert_eq!(leg.message, "Submitted");
        }
        assert_eq!(
            terminal.wallet_clusters.status,
            Some(("Submitted 2 cluster legs for BTC".into(), false))
        );
        assert_eq!(
            terminal.spot_balances_revision,
            revision + u64::from(market_type == MarketType::Spot)
        );
        assert_eq!(
            terminal
                .account_data
                .as_ref()
                .expect("account data")
                .completeness
                .spot_balances_complete,
            market_type != MarketType::Spot
        );
    }
}

#[test]
fn cluster_dispatch_rejects_empty_preparation_without_allocating_an_execution() {
    let mut terminal = terminal_with_execution();
    let next_id = terminal.wallet_clusters.next_execution_id;
    let task = terminal.start_wallet_cluster_execution(
        WalletCluster::default(),
        WalletClusterExecutionKind::Order,
        "BTC".into(),
        OrderKind::Market,
        vec![],
    );
    assert_eq!(task.units(), 0);
    assert_eq!(terminal.wallet_clusters.next_execution_id, next_id);
    assert_eq!(terminal.wallet_clusters.executions.len(), 1);
    assert_eq!(terminal.wallet_clusters.executions[0].id, 7);
    assert_eq!(
        terminal.wallet_clusters.status,
        Some(("No eligible cluster members to submit".into(), true))
    );
}

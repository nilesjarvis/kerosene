use super::*;
use crate::config::KeroseneConfig;
use crate::order_execution::OrderSurface;
use crate::signing::ExchangeOrderKind;
use serde_json::json;

const ADDRESS: &str = "0x1111111111111111111111111111111111111111";

fn terminal_with_execution() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal
        .wallet_clusters
        .push_execution(WalletClusterExecution {
            id: 7,
            cluster_name: "Test cluster".into(),
            kind: WalletClusterExecutionKind::Order,
            symbol: "BTC".into(),
            order_kind: OrderKind::Limit,
            created_at_ms: 42,
            legs: ["first", "second"]
                .into_iter()
                .map(|id| WalletClusterExecutionLeg {
                    profile_secret_id: id.into(),
                    address: ADDRESS.into(),
                    label: id.into(),
                    symbol: "BTC".into(),
                    is_buy: true,
                    size: "1".into(),
                    price: "100".into(),
                    cloid: format!("cloid-{id}"),
                    status: WalletClusterLegStatus::Pending,
                    message: "Submitted".into(),
                })
                .collect(),
        });
    terminal
}

fn context(order_kind: ExchangeOrderKind) -> OneShotPlacementContext {
    OneShotPlacementContext {
        account_address: ADDRESS.into(),
        cloid: "cloid-first".into(),
        surface: OrderSurface::Cluster,
        symbol_key: "BTC".into(),
        order_kind,
    }
}

fn response(status: serde_json::Value) -> Result<ExchangeResponse, String> {
    Ok(serde_json::from_value(json!({
        "status": "ok", "response": {"type": "order", "data": {"statuses": [status]}}
    }))
    .expect("exchange response fixture"))
}

#[test]
fn cluster_order_results_preserve_accepted_rejected_and_uncertain_outcomes() {
    use ExchangeOrderKind::{Limit, LimitIoc, Market};
    use WalletClusterLegStatus::{Checking, Confirmed, Failed, Uncertain};

    for (kind, result, expected) in [
        (Limit, response(json!({"resting": {"oid": 1}})), Confirmed),
        (Market, response(json!({"resting": {"oid": 1}})), Uncertain),
        (
            LimitIoc,
            response(json!({"resting": {"oid": 1}})),
            Uncertain,
        ),
        (
            Market,
            response(json!({"filled": {"totalSz": "1", "avgPx": "100", "oid": 2}})),
            Confirmed,
        ),
        (
            Limit,
            response(json!({"error": "Rejected api_key=fixture-secret"})),
            Failed,
        ),
        (Limit, response(json!("success")), Checking),
        (
            Limit,
            Err("Connection lost api_key=fixture-secret".into()),
            Checking,
        ),
    ] {
        let mut terminal = terminal_with_execution();
        let _ = terminal.update_wallet_cluster(Message::WalletClusterOrderResult {
            execution_id: 7,
            member_key: Some("first".to_string()).into(),
            context: context(kind),
            result: Box::new(result),
        });
        let execution = &terminal.wallet_clusters.executions[0];
        assert_eq!(execution.legs[0].status, expected);
        assert!(!execution.legs[0].message.contains("fixture-secret"));
        if matches!(expected, Failed | Checking) && execution.legs[0].message.contains("api_key") {
            assert!(execution.legs[0].message.contains("api_key=<redacted>"));
        }
        assert_eq!(execution.legs[1].status, WalletClusterLegStatus::Pending);
        assert_eq!(execution.legs[1].message, "Submitted");
        assert!(terminal.wallet_clusters.has_pending_execution());
        let (message, is_error) = terminal.wallet_clusters.status.as_ref().expect("progress");
        assert_eq!(
            message,
            if expected == Checking {
                "Cluster execution progress: 0/2 legs finished"
            } else {
                "Cluster execution progress: 1/2 legs finished"
            }
        );
        assert_eq!(*is_error, matches!(expected, Failed | Uncertain));
    }
}

#[test]
fn cluster_status_results_keep_ambiguous_answers_uncertain_and_redact_errors() {
    use ExchangeOrderKind::{Limit, LimitIoc, Market};
    use WalletClusterLegStatus::{Confirmed, Failed, Uncertain};

    for (kind, status, expected) in [
        (Limit, "open", Confirmed),
        (Market, "open", Uncertain),
        (LimitIoc, "open", Uncertain),
        (Market, "filled", Confirmed),
        (Limit, "rejected", Failed),
        (Limit, "canceled", Uncertain),
        (Limit, "unknownOid", Uncertain),
        (Limit, "transport error", Uncertain),
    ] {
        let mut terminal = terminal_with_execution();
        let result = if status == "transport error" {
            Err("Status failed api_key=fixture-secret".into())
        } else {
            Ok(OrderStatusResult {
                status: status.into(),
                oid: Some(1),
                cloid: Some("cloid-first".into()),
                raw_summary: format!("status: {status}"),
            })
        };
        let _ = terminal.update_wallet_cluster(Message::WalletClusterOrderStatusLoaded {
            execution_id: 7,
            member_key: Some("first".to_string()).into(),
            context: context(kind),
            result: Box::new(result),
        });
        let execution = &terminal.wallet_clusters.executions[0];
        assert_eq!(execution.legs[0].status, expected);
        assert_eq!(execution.legs[1].status, WalletClusterLegStatus::Pending);
        assert_eq!(execution.legs[1].message, "Submitted");
        if status == "transport error" {
            assert_eq!(
                execution.legs[0].message,
                "Status failed api_key=<redacted>"
            );
        } else {
            assert!(
                execution.legs[0]
                    .message
                    .contains(&format!("status: {status}"))
            );
        }
    }
}

#[test]
fn cluster_result_updates_require_matching_execution_member_and_cloid() {
    for (execution_id, member, cloid) in [
        (8, Some("first"), "cloid-first"),
        (7, None, "cloid-first"),
        (7, Some("missing"), "cloid-first"),
        (7, Some("first"), "cloid-second"),
    ] {
        for status_lookup in [false, true] {
            let mut terminal = terminal_with_execution();
            let mut context = context(ExchangeOrderKind::Limit);
            context.cloid = cloid.into();
            let member_key = member.map(str::to_string).into();
            let message = if status_lookup {
                Message::WalletClusterOrderStatusLoaded {
                    execution_id,
                    member_key,
                    context,
                    result: Box::new(Ok(OrderStatusResult {
                        status: "filled".into(),
                        oid: Some(1),
                        cloid: Some(cloid.into()),
                        raw_summary: "Filled".into(),
                    })),
                }
            } else {
                Message::WalletClusterOrderResult {
                    execution_id,
                    member_key,
                    context,
                    result: Box::new(response(json!({"resting": {"oid": 1}}))),
                }
            };
            let _ = terminal.update_wallet_cluster(message);
            assert_eq!(terminal.wallet_clusters.executions.len(), 1);
            for leg in &terminal.wallet_clusters.executions[0].legs {
                assert_eq!(leg.status, WalletClusterLegStatus::Pending);
                assert_eq!(leg.message, "Submitted");
            }
        }
    }
}

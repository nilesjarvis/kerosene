use super::*;
use crate::account::transfers::TransferHistoryKind;
use serde_json::{Value, json};

fn parse(delta: Value, account: &str) -> Result<Option<TransferEntry>, String> {
    super::super::ledger_entry(
        LedgerEntry {
            time: 123,
            hash: "synthetic-ledger-hash".to_string(),
            delta,
        },
        account,
    )
}

fn spot() -> Value {
    json!({
        "type": "spotTransfer", "user": "SENDER", "destination": "RECIPIENT",
        "token": "UBTC", "amount": "0.123456789012345678", "fee": "0.1", "feeToken": "USDC"
    })
}

#[test]
fn spot_transfers_preserve_precision_and_classify_both_sides_case_insensitively() {
    for (account, direction) in [
        ("sender", TransferDirection::Sent),
        ("recipient", TransferDirection::Received),
    ] {
        let entry = parse(spot(), account).expect("valid").expect("transfer");
        assert_eq!(entry.direction, direction);
        assert_eq!(entry.amount, "0.123456789012345678");
        assert_eq!(entry.asset, "UBTC");
        assert_eq!(entry.source_address.as_deref(), Some("SENDER"));
        assert_eq!(entry.destination_address.as_deref(), Some("RECIPIENT"));
        assert_eq!(entry.fee.as_deref(), Some("0.1"));
        assert_eq!(entry.fee_asset.as_deref(), Some("USDC"));
        assert!(TransferHistoryKind::Transfers.includes(&entry));
        assert!(!TransferHistoryKind::DepositsWithdrawals.includes(&entry));
    }
    assert!(parse(spot(), "unrelated").expect("ignored").is_none());
}

#[test]
fn usdc_transfers_include_spot_perps_and_subaccounts() {
    for kind in ["spotTransfer", "internalTransfer", "subAccountTransfer"] {
        for (account, direction) in [
            ("sender", TransferDirection::Sent),
            ("recipient", TransferDirection::Received),
        ] {
            let entry = parse(
                json!({
                    "type": kind, "user": "sender", "destination": "recipient",
                    "token": "USDC", "amount": "123.456789", "usdc": "123.456789"
                }),
                account,
            )
            .expect("valid")
            .expect("transfer");
            assert_eq!(entry.direction, direction);
            assert_eq!(entry.asset, "USDC");
            assert_eq!(entry.amount, "123.456789");
        }
    }
}

#[test]
fn internal_movements_show_spot_perps_route_without_inventing_an_external_wallet() {
    for (to_perp, source, destination) in [(true, "Spot", "Perps"), (false, "Perps", "Spot")] {
        let entry = parse(
            json!({"type": "accountClassTransfer", "usdc": 25, "toPerp": to_perp}),
            "account",
        )
        .expect("valid")
        .expect("transfer");
        assert_eq!(entry.direction, TransferDirection::Internal);
        assert_eq!(entry.amount, "25");
        assert_eq!(entry.source_chain, source);
        assert_eq!(entry.destination_chain, destination);
        assert_eq!(entry.source_address, entry.destination_address);
        assert_eq!(entry.source_address.as_deref(), Some("account"));
    }
    let mut delta = spot();
    delta["destination"] = json!("sender");
    assert_eq!(
        parse(delta, "sender")
            .expect("valid")
            .expect("self transfer")
            .direction,
        TransferDirection::Internal
    );
}

#[test]
fn transfers_do_not_include_bridges_or_other_ledger_categories() {
    for kind in ["deposit", "withdraw"] {
        let entry = parse(json!({"type": kind, "usdc": "10"}), "account")
            .expect("valid")
            .expect("bridge");
        assert!(!TransferHistoryKind::Transfers.includes(&entry));
        assert!(TransferHistoryKind::DepositsWithdrawals.includes(&entry));
    }
    for kind in [
        "vaultDeposit",
        "funding",
        "spotGenesis",
        "rewardsClaim",
        "unknown",
    ] {
        assert!(
            parse(json!({"type": kind}), "account")
                .expect("ignored")
                .is_none()
        );
    }
}

#[test]
fn malformed_transfers_fail_without_exposing_upstream_values() {
    for field in ["user", "destination", "token", "amount"] {
        let mut delta = spot();
        delta[field] = Value::Null;
        let error = parse(delta, "sender").expect_err("missing field");
        assert!(!error.contains("SENDER"));
    }
    for amount in ["NaN", "-1", "1e10", ""] {
        let mut delta = spot();
        delta["amount"] = json!(amount);
        assert!(parse(delta, "sender").is_err());
    }
    assert!(
        parse(
            json!({"type": "accountClassTransfer", "usdc": "1"}),
            "account"
        )
        .is_err()
    );
}

#[test]
fn ids_deduplicate_boundary_replays_without_collapsing_distinct_transfers() {
    let first = parse(spot(), "sender").expect("valid").expect("transfer");
    assert_eq!(
        first.id,
        parse(spot(), "sender").expect("valid").expect("replay").id
    );
    for (field, value) in [
        ("token", "HYPE"),
        ("amount", "1"),
        ("destination", "another-wallet"),
    ] {
        let mut delta = spot();
        delta[field] = json!(value);
        assert_ne!(
            first.id,
            parse(delta, "sender").expect("valid").expect("distinct").id
        );
    }
    let debug = format!(
        "{:?}",
        crate::message::Message::TransferHistoryLoaded(
            "sender".into(),
            1,
            TransferProvider::Hyperliquid,
            Box::new(Ok(crate::account::transfers::TransferSnapshot {
                entries: vec![first],
                ..Default::default()
            }))
        )
    );
    for sensitive in [
        "sender",
        "SENDER",
        "RECIPIENT",
        "synthetic-ledger-hash",
        "0.123456789012345678",
        "UBTC",
    ] {
        assert!(!debug.contains(sensitive));
    }
}

#[test]
fn legacy_spot_fees_do_not_inherit_the_transferred_asset() {
    let mut delta = spot();
    delta["feeToken"] = json!("");
    let entry = parse(delta, "sender").expect("valid").expect("transfer");
    assert_eq!(entry.fee.as_deref(), Some("0.1"));
    assert!(entry.fee_asset.is_none());
}

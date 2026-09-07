use super::{
    TransferDirection, TransferEntry, TransferProvider, TransferSnapshot, chain_label,
    decimal_units, nonempty,
};
use chrono::DateTime;
use serde::Deserialize;
use std::collections::HashSet;

const UNIT_API: &str = "https://api.hyperunit.xyz";

#[derive(Deserialize)]
struct Operations {
    // No default: an error object must not look like a successful empty history.
    operations: Vec<Operation>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Operation {
    #[serde(default)]
    operation_id: String,
    op_created_at: String,
    #[serde(default)]
    protocol_address: String,
    #[serde(default)]
    source_address: String,
    #[serde(default)]
    destination_address: String,
    source_chain: String,
    destination_chain: String,
    source_amount: String,
    #[serde(default)]
    destination_fee_amount: String,
    #[serde(default)]
    sweep_fee_amount: String,
    #[serde(default)]
    source_tx_hash: String,
    #[serde(default)]
    destination_tx_hash: String,
    source_tx_confirmations: Option<u64>,
    destination_tx_confirmations: Option<u64>,
    asset: String,
    state: String,
}

pub(crate) async fn fetch_unit_history(address: String) -> Result<TransferSnapshot, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(25))
        .build()
        .map_err(|_| "Could not create Unit history client".to_string())?;
    // The URL contains the account address. Do not interpolate reqwest errors.
    let response = client
        .get(format!("{UNIT_API}/operations/{address}"))
        .send()
        .await
        .map_err(|_| "Unit history request failed".to_string())?;
    if !response.status().is_success() {
        return Err(format!("Unit history returned HTTP {}", response.status()));
    }
    let raw = response
        .text()
        .await
        .map_err(|_| "Could not read Unit history".to_string())?;
    parse_operations(&raw, &address)
}

fn parse_operations(raw: &str, account: &str) -> Result<TransferSnapshot, String> {
    let response: Operations = serde_json::from_str(raw)
        .map_err(|_| "Unit returned an invalid history response".to_string())?;
    let mut snapshot = TransferSnapshot::default();
    let mut seen = HashSet::new();
    for op in response.operations {
        let direction = match (op.source_chain.as_str(), op.destination_chain.as_str()) {
            ("hyperliquid", destination) if destination != "hyperliquid" => {
                TransferDirection::Withdrawal
            }
            (source, "hyperliquid") if source != "hyperliquid" => TransferDirection::Deposit,
            _ => continue,
        };
        let account_side = match direction {
            TransferDirection::Deposit => &op.destination_address,
            TransferDirection::Withdrawal => &op.source_address,
        };
        // Discovered deposits can omit the destination until an operation is
        // created. The address-scoped API still associates them with this user.
        if !account_side.is_empty() && !account_side.eq_ignore_ascii_case(account) {
            continue;
        }
        let Some(time) = DateTime::parse_from_rfc3339(&op.op_created_at)
            .ok()
            .and_then(|time| u64::try_from(time.timestamp_millis()).ok())
        else {
            snapshot.warning = Some("Some Unit operations have invalid timestamps".to_string());
            continue;
        };
        let id = if op.operation_id.is_empty() {
            format!(
                "unit:{}:{}:{}",
                op.source_tx_hash, op.protocol_address, op.asset
            )
        } else {
            format!("unit:{}", op.operation_id)
        };
        if !seen.insert(id.clone()) {
            continue;
        }
        let decimals = native_decimals(&op.asset);
        let amount = display_amount(&op.source_amount, decimals);
        let (status, failed) = operation_status(&op.state);
        snapshot.entries.push(TransferEntry {
            id,
            time,
            provider: TransferProvider::Unit,
            direction,
            asset: if op.asset == "fart" {
                "FARTCOIN".to_string()
            } else {
                op.asset.to_uppercase()
            },
            amount,
            source_chain: chain_label(&op.source_chain),
            destination_chain: chain_label(&op.destination_chain),
            source_address: nonempty(op.source_address),
            destination_address: nonempty(op.destination_address),
            protocol_address: nonempty(op.protocol_address),
            source_tx: nonempty(op.source_tx_hash),
            destination_tx: nonempty(op.destination_tx_hash),
            ledger_tx: None,
            status,
            failed,
            fee: nonempty(op.destination_fee_amount).map(|raw| display_amount(&raw, decimals)),
            sweep_fee: nonempty(op.sweep_fee_amount).map(|raw| display_amount(&raw, decimals)),
            source_confirmations: op.source_tx_confirmations,
            destination_confirmations: op.destination_tx_confirmations,
        });
    }
    Ok(snapshot)
}

// Native units from Unit's operation contract and official frontend asset
// registry, not HyperCore weiDecimals (UBTC=10, UETH=9 differ from BTC/ETH).
fn native_decimals(asset: &str) -> Option<usize> {
    match asset {
        "btc" | "zec" | "2z" | "spx" | "spxs" | "spyx" => Some(8),
        "eth" | "ena" | "xpl" | "mon" | "avax" | "virtual" => Some(18),
        "sol" => Some(9),
        "pump" | "fart" | "fartcoin" | "ansem" | "trump" => Some(6),
        "bonk" => Some(5),
        _ => None,
    }
}

fn display_amount(raw: &str, decimals: Option<usize>) -> String {
    match decimals {
        Some(decimals) => decimal_units(raw, decimals).unwrap_or_else(|| "Unavailable".to_string()),
        None => decimal_units(raw, 0)
            .map(|value| format!("{value} base units"))
            .unwrap_or_else(|| "Unavailable".to_string()),
    }
}

fn operation_status(state: &str) -> (String, bool) {
    let label = match state {
        "sourceTxDiscovered" | "srcTxDiscovered" => "Discovered",
        "waitForSrcTxFinalization" => "Confirming source",
        "buildingDstTx" => "Preparing delivery",
        "additionalChecks" => "Additional checks",
        "signTx" => "Signing",
        "broadcastTx" => "Broadcasting",
        "waitForDstTxFinalization" => "Confirming delivery",
        "readyForWithdrawQueue" | "queuedForWithdraw" => "Withdrawal queued",
        "waitForSweep" | "queuedForSweep" => "Sweep pending",
        "done" => "Completed",
        "failure" => "Failed",
        _ => return (format!("Unknown ({state})"), false),
    };
    (label.to_string(), state == "failure")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

    fn operation() -> Value {
        json!({
            "operationId":"synthetic-source:0", "opCreatedAt":"2026-09-01T12:00:00.123Z",
            "protocolAddress":"synthetic-btc-bridge", "sourceAddress":"synthetic-btc-sender",
            "destinationAddress":ACCOUNT, "sourceChain":"bitcoin", "destinationChain":"hyperliquid",
            "sourceAmount":"10000000", "destinationFeeAmount":"123", "sweepFeeAmount":"456.5",
            "sourceTxHash":"synthetic-source:0", "destinationTxHash":"synthetic-guardian:42",
            "asset":"btc", "state":"done"
        })
    }

    fn parse(operations: Vec<Value>) -> TransferSnapshot {
        parse_operations(&json!({"operations":operations}).to_string(), ACCOUNT)
            .expect("valid operations")
    }

    #[test]
    fn bitcoin_deposit_keeps_source_wallet_separate_from_bridge_and_destination() {
        let snapshot = parse(vec![operation()]);
        let entry = &snapshot.entries[0];
        assert_eq!(entry.direction, TransferDirection::Deposit);
        assert_eq!(entry.amount, "0.1");
        assert_eq!(entry.time, 1788264000123);
        assert_eq!(
            entry.source_address.as_deref(),
            Some("synthetic-btc-sender")
        );
        assert_eq!(
            entry.protocol_address.as_deref(),
            Some("synthetic-btc-bridge")
        );
        assert_eq!(entry.destination_address.as_deref(), Some(ACCOUNT));
        assert_eq!(entry.fee.as_deref(), Some("0.00000123"));
        assert_eq!(entry.sweep_fee.as_deref(), Some("0.000004565"));
        assert_eq!(entry.source_tx.as_deref(), Some("synthetic-source:0"));
        assert_eq!(
            entry.destination_tx.as_deref(),
            Some("synthetic-guardian:42")
        );
        assert_eq!(entry.status, "Completed");
        assert!(entry.source_confirmations.is_none());
        let debug = format!(
            "{:?}",
            crate::message::Message::TransferHistoryLoaded(
                ACCOUNT.into(),
                1,
                TransferProvider::Unit,
                Box::new(Ok(snapshot))
            )
        );
        for sensitive in [
            ACCOUNT,
            "synthetic-btc-sender",
            "synthetic-btc-bridge",
            "synthetic-source",
            "synthetic-guardian",
        ] {
            assert!(!debug.contains(sensitive));
        }
    }

    #[test]
    fn withdrawals_use_native_decimals_and_preserve_external_destination() {
        let mut op = operation();
        op["sourceChain"] = json!("hyperliquid");
        op["destinationChain"] = json!("ethereum");
        op["sourceAddress"] = json!(ACCOUNT);
        op["destinationAddress"] = json!("synthetic-ethereum-recipient");
        op["asset"] = json!("eth");
        op["sourceAmount"] = json!("100000000000000001");
        op["state"] = json!("queuedForWithdraw");
        op["sourceTxConfirmations"] = json!(200);
        op["destinationTxConfirmations"] = json!(0);
        let snapshot = parse(vec![op]);
        let entry = &snapshot.entries[0];
        assert_eq!(entry.direction, TransferDirection::Withdrawal);
        assert_eq!(entry.amount, "0.100000000000000001");
        assert_eq!(entry.source_chain, "Hyperliquid");
        assert_eq!(entry.destination_chain, "Ethereum");
        assert_eq!(
            entry.destination_address.as_deref(),
            Some("synthetic-ethereum-recipient")
        );
        assert_eq!(entry.status, "Withdrawal queued");
        assert_eq!(entry.source_confirmations, Some(200));
        assert_eq!(entry.destination_confirmations, Some(0));
    }

    #[test]
    fn unconfirmed_deposits_unknown_assets_and_future_states_are_visible() {
        let mut op = operation();
        op["operationId"] = json!("");
        op["destinationAddress"] = json!("");
        op["destinationTxHash"] = json!("");
        op["asset"] = json!("future-token");
        op["state"] = json!("futureState");
        let snapshot = parse(vec![op]);
        let entry = &snapshot.entries[0];
        assert_eq!(entry.amount, "10000000 base units");
        assert_eq!(entry.status, "Unknown (futureState)");
        assert!(entry.destination_address.is_none());
        assert!(entry.destination_tx.is_none());
    }

    #[test]
    fn duplicate_operations_are_removed_but_distinct_outputs_survive() {
        let mut second = operation();
        second["operationId"] = json!("synthetic-source:1");
        second["sourceTxHash"] = json!("synthetic-source:1");
        assert_eq!(
            parse(vec![operation(), operation(), second]).entries.len(),
            2
        );
    }

    #[test]
    fn error_payloads_are_errors_and_invalid_timestamps_are_flagged() {
        assert!(parse_operations("{}", ACCOUNT).is_err());
        assert!(parse_operations(r#"{"error":"unavailable"}"#, ACCOUNT).is_err());
        let mut op = operation();
        op["opCreatedAt"] = json!("invalid");
        let snapshot = parse(vec![op]);
        assert!(snapshot.entries.is_empty());
        assert!(snapshot.warning.is_some());
    }

    #[test]
    fn wrong_accounts_and_non_hyperliquid_routes_are_excluded() {
        let mut wrong_account = operation();
        wrong_account["destinationAddress"] = json!("another-account");
        let mut other_route = operation();
        other_route["destinationChain"] = json!("ethereum");
        assert!(parse(vec![wrong_account, other_route]).entries.is_empty());
        assert_eq!(operation_status("failure"), ("Failed".into(), true));
    }

    #[test]
    fn all_verified_native_asset_decimals_are_independent_of_hypercore_precision() {
        for (asset, decimals) in [
            ("btc", 8),
            ("eth", 18),
            ("sol", 9),
            ("pump", 6),
            ("fart", 6),
            ("bonk", 5),
            ("2z", 8),
            ("spxs", 8),
            ("xpl", 18),
            ("mon", 18),
            ("zec", 8),
            ("avax", 18),
            ("virtual", 18),
            ("ena", 18),
            ("ansem", 6),
        ] {
            assert_eq!(native_decimals(asset), Some(decimals));
        }
        assert_eq!(native_decimals("unknown"), None);
    }
}

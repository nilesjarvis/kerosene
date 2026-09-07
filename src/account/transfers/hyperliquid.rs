use super::{TransferDirection, TransferEntry, TransferProvider, TransferSnapshot, decimal_units};
use crate::api::{API_URL, proxy::HyperliquidRequestExt};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Deserialize)]
struct LedgerEntry {
    time: u64,
    hash: String,
    delta: Value,
}

pub(crate) async fn fetch_bridge_history(
    address: String,
    start: u64,
    end: u64,
) -> Result<TransferSnapshot, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|_| "Could not create Hyperliquid history client".to_string())?;
    let mut snapshot = TransferSnapshot::default();
    let mut cursor = start;
    let mut seen = HashSet::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);
    // All ledger categories count toward pagination, even though this tab only
    // displays bridge deposits/withdrawals. Overlap the boundary and deduplicate.
    for _ in 0..100 {
        let page = tokio::time::timeout_at(deadline, async {
            let response = client
                .post(API_URL)
                .json(&ledger_request(&address, cursor, end))
                .send_info()
                .await
                .map_err(|_| "Hyperliquid bridge history request failed".to_string())?;
            if !response.status().is_success() {
                return Err(format!(
                    "Hyperliquid bridge history returned HTTP {}",
                    response.status()
                ));
            }
            response
                .json::<Vec<LedgerEntry>>()
                .await
                .map_err(|_| "Hyperliquid returned an invalid ledger response".to_string())
        })
        .await
        .unwrap_or_else(|_| Err("Hyperliquid bridge history request timed out".to_string()));
        let page = match page {
            Ok(page) => page,
            Err(error) if snapshot.entries.is_empty() => return Err(error),
            Err(error) => {
                snapshot.warning = Some(format!("Partial history: {error}. Refresh to continue."));
                snapshot.next_start = Some(cursor);
                return Ok(snapshot);
            }
        };
        let last_time = page.iter().map(|entry| entry.time).max();
        let page_len = page.len();
        for entry in page {
            if entry.time < cursor || entry.time > end {
                continue;
            }
            if let Some(transfer) = bridge_entry(entry, &address)?
                && seen.insert(transfer.id.clone())
            {
                snapshot.entries.push(transfer);
            }
        }
        match next_cursor(cursor, last_time, page_len) {
            Ok(Some(next)) => cursor = next,
            Ok(None) => {
                snapshot.next_start = Some(end.saturating_sub(60_000));
                return Ok(snapshot);
            }
            Err(warning) => {
                snapshot.warning = Some(warning.to_string());
                return Ok(snapshot);
            }
        }
    }
    snapshot.warning = Some("Partial Hyperliquid history: refresh to continue loading".to_string());
    snapshot.next_start = Some(cursor);
    Ok(snapshot)
}

fn ledger_request(address: &str, start: u64, end: u64) -> Value {
    json!({"type": "userNonFundingLedgerUpdates", "user": address, "startTime": start, "endTime": end})
}

fn next_cursor(cursor: u64, last: Option<u64>, count: usize) -> Result<Option<u64>, &'static str> {
    match last {
        None => Ok(None),
        Some(last) if last > cursor => Ok(Some(last)),
        // A full page that cannot advance may hide additional entries at the
        // same millisecond; do not claim a complete history or skip that block.
        Some(_) if count >= 500 => {
            Err("Partial Hyperliquid history: timestamp page could not advance")
        }
        Some(_) => Ok(None),
    }
}

fn bridge_entry(entry: LedgerEntry, account: &str) -> Result<Option<TransferEntry>, String> {
    let direction = match entry.delta["type"].as_str() {
        Some("deposit") => TransferDirection::Deposit,
        Some("withdraw") => TransferDirection::Withdrawal,
        _ => return Ok(None),
    };
    let amount = ledger_number(&entry.delta["usdc"])
        .ok_or_else(|| "Hyperliquid returned an invalid bridge amount".to_string())?;
    let deposit = direction == TransferDirection::Deposit;
    Ok(Some(TransferEntry {
        id: format!(
            "hl:{}:{}:{}:{amount}",
            entry.hash,
            entry.time,
            direction.label()
        ),
        time: entry.time,
        provider: TransferProvider::Hyperliquid,
        direction,
        asset: "USDC".to_string(),
        amount,
        source_chain: if deposit { "Arbitrum" } else { "Hyperliquid" }.to_string(),
        destination_chain: if deposit { "Hyperliquid" } else { "Arbitrum" }.to_string(),
        // The ledger does not expose the Arbitrum sender/recipient. In
        // particular a withdrawal can target a different wallet from account.
        source_address: (!deposit).then(|| account.to_string()),
        destination_address: deposit.then(|| account.to_string()),
        protocol_address: None,
        source_tx: None,
        destination_tx: None,
        ledger_tx: super::nonempty(entry.hash),
        // A ledger withdrawal is a debit, not proof of Arbitrum finalization.
        status: if deposit { "Credited" } else { "Debited" }.to_string(),
        failed: false,
        fee: ledger_number(&entry.delta["fee"]),
        sweep_fee: None,
        source_confirmations: None,
        destination_confirmations: None,
    }))
}

fn ledger_number(value: &Value) -> Option<String> {
    match value {
        Value::String(raw) => decimal_units(raw, 0),
        Value::Number(raw) => decimal_units(&raw.to_string(), 0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ledger_request_uses_inclusive_range_and_actual_account() {
        assert_eq!(
            ledger_request("account", 0, 900),
            json!({
                "type":"userNonFundingLedgerUpdates", "user":"account", "startTime":0, "endTime":900
            })
        );
    }

    #[test]
    fn pagination_overlaps_timestamps_and_detects_stalled_full_pages() {
        assert_eq!(next_cursor(10, Some(20), 500), Ok(Some(20)));
        assert_eq!(next_cursor(20, Some(20), 1), Ok(None));
        assert_eq!(next_cursor(20, None, 0), Ok(None));
        assert!(next_cursor(20, Some(20), 500).is_err());
    }

    #[test]
    fn native_bridges_preserve_amounts_without_inventing_wallets_or_finality() {
        for (kind, amount) in [("deposit", json!("100.123456")), ("withdraw", json!(100))] {
            let entry = bridge_entry(
                LedgerEntry {
                    time: 100,
                    hash: "ledger".into(),
                    delta: json!({"type":kind, "usdc":amount, "fee":"1"}),
                },
                "account",
            )
            .expect("valid bridge")
            .expect("bridge row");
            assert_eq!(entry.asset, "USDC");
            assert_eq!(entry.fee.as_deref(), Some("1"));
            assert!(entry.source_tx.is_none());
            assert!(entry.destination_tx.is_none());
            if kind == "deposit" {
                assert_eq!(entry.amount, "100.123456");
                assert!(entry.source_address.is_none());
                assert_eq!(entry.destination_address.as_deref(), Some("account"));
            } else {
                assert_eq!(entry.status, "Debited");
                assert!(entry.destination_address.is_none());
            }
        }
    }

    #[test]
    fn internal_transfers_are_not_external_deposits() {
        for kind in [
            "spotTransfer",
            "internalTransfer",
            "accountClassTransfer",
            "vaultDeposit",
            "subAccountTransfer",
        ] {
            assert!(
                bridge_entry(
                    LedgerEntry {
                        time: 1,
                        hash: "test".into(),
                        delta: json!({"type":kind})
                    },
                    "account"
                )
                .expect("ignored event")
                .is_none()
            );
        }
    }

    #[test]
    fn invalid_bridge_amount_is_an_error_instead_of_zero() {
        assert!(
            bridge_entry(
                LedgerEntry {
                    time: 1,
                    hash: "test".into(),
                    delta: json!({"type":"deposit", "usdc":"NaN"})
                },
                "account"
            )
            .is_err()
        );
    }
}

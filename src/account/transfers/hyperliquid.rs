use super::{TransferDirection, TransferEntry, TransferProvider, TransferSnapshot, decimal_units};
use crate::api::{API_URL, CLIENT, proxy::HyperliquidRequestExt};
use futures::{SinkExt, Stream};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashSet;

mod token_transfers;

const LEDGER_PAGE_SIZE: usize = 500;
const MAX_PAGES: usize = 100;

#[derive(Deserialize)]
struct LedgerEntry {
    time: u64,
    hash: String,
    delta: Value,
}

pub(crate) fn fetch_ledger_history(
    address: String,
    start: u64,
    end: u64,
) -> impl Stream<Item = Result<TransferSnapshot, String>> {
    let request_address = address.clone();
    ledger_history(address, start, end, move |cursor| {
        fetch_ledger_page(request_address.clone(), cursor, end)
    })
}

async fn fetch_ledger_page(
    address: String,
    start: u64,
    end: u64,
) -> Result<Vec<LedgerEntry>, String> {
    let response = CLIENT
        .post(API_URL)
        .timeout(std::time::Duration::from_secs(20))
        .json(&ledger_request(&address, start, end))
        .send_info()
        .await
        .map_err(|_| "Hyperliquid ledger history request failed".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Hyperliquid ledger history returned HTTP {}",
            response.status()
        ));
    }
    response
        .json::<Vec<LedgerEntry>>()
        .await
        .map_err(|_| "Hyperliquid returned an invalid ledger response".to_string())
}

fn ledger_history<F, Fut>(
    address: String,
    start: u64,
    end: u64,
    mut fetch_page: F,
) -> impl Stream<Item = Result<TransferSnapshot, String>>
where
    F: FnMut(u64) -> Fut,
    Fut: Future<Output = Result<Vec<LedgerEntry>, String>>,
{
    iced::stream::channel(1, async move |mut output| {
        let mut cursor = start;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);
        // Publish each page before waiting for the next request's read budget.
        // Both tabs share this cursor, including pages with no matching rows.
        for page_index in 0..MAX_PAGES {
            let page = tokio::time::timeout_at(deadline, fetch_page(cursor))
                .await
                .unwrap_or_else(
                    |_| Err("Hyperliquid ledger history request timed out".to_string()),
                );
            let page = match page {
                Ok(page) => page,
                Err(error) => {
                    let _ = output.send(Err(error)).await;
                    return;
                }
            };
            let last_time = page.iter().map(|entry| entry.time).max();
            let page_len = page.len();
            let mut snapshot = TransferSnapshot::default();
            let mut seen = HashSet::new();
            for entry in page {
                if entry.time < cursor || entry.time > end {
                    continue;
                }
                match ledger_entry(entry, &address) {
                    Ok(Some(transfer)) if seen.insert(transfer.id.clone()) => {
                        snapshot.entries.push(transfer);
                    }
                    Ok(_) => {}
                    Err(error) => {
                        let _ = output.send(Err(error)).await;
                        return;
                    }
                }
            }
            match next_cursor(cursor, last_time, page_len) {
                Ok(Some(next)) => {
                    cursor = next;
                    snapshot.next_start = Some(cursor);
                    snapshot.has_more = page_index + 1 < MAX_PAGES;
                    if !snapshot.has_more {
                        snapshot.warning = Some(
                            "Partial Hyperliquid history: refresh to continue loading".to_string(),
                        );
                    }
                }
                Ok(None) => snapshot.next_start = Some(end.saturating_sub(60_000)),
                Err(warning) => {
                    snapshot.warning = Some(warning.to_string());
                    snapshot.next_start = Some(cursor);
                }
            }
            let has_more = snapshot.has_more;
            if output.send(Ok(snapshot)).await.is_err() || !has_more {
                return;
            }
        }
    })
}

fn ledger_request(address: &str, start: u64, end: u64) -> Value {
    json!({"type": "userNonFundingLedgerUpdates", "user": address, "startTime": start, "endTime": end})
}

fn next_cursor(cursor: u64, last: Option<u64>, count: usize) -> Result<Option<u64>, &'static str> {
    // A short page exhausts the requested range. Probing its last timestamp
    // again adds another round trip and may wait for a whole budget window.
    if count < LEDGER_PAGE_SIZE {
        return Ok(None);
    }
    match last {
        None => Ok(None),
        Some(last) if last > cursor => Ok(Some(last)),
        // A full page that cannot advance may hide additional entries at the
        // same millisecond; do not claim a complete history or skip that block.
        Some(_) => Err("Partial Hyperliquid history: timestamp page could not advance"),
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
        fee_asset: Some("USDC".to_string()),
        sweep_fee: None,
        source_confirmations: None,
        destination_confirmations: None,
    }))
}

fn ledger_entry(entry: LedgerEntry, account: &str) -> Result<Option<TransferEntry>, String> {
    if matches!(entry.delta["type"].as_str(), Some("deposit" | "withdraw")) {
        bridge_entry(entry, account)
    } else {
        token_transfers::transfer_entry(entry, account)
    }
}

fn ledger_number(value: &Value) -> Option<String> {
    match value {
        Value::String(raw) => decimal_units(raw, 0),
        Value::Number(raw) => decimal_units(&raw.to_string(), 0),
        _ => None,
    }
}

#[cfg(test)]
mod tests;

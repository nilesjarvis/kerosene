use super::{LedgerEntry, TransferDirection, TransferEntry, TransferProvider, ledger_number};

pub(super) fn transfer_entry(
    entry: LedgerEntry,
    account: &str,
) -> Result<Option<TransferEntry>, String> {
    let delta = &entry.delta;
    let kind = delta["type"].as_str().unwrap_or_default();
    let spot = kind == "spotTransfer";
    let account_class = kind == "accountClassTransfer";
    if !matches!(
        kind,
        "spotTransfer" | "internalTransfer" | "subAccountTransfer" | "accountClassTransfer"
    ) {
        return Ok(None);
    }

    let invalid = || "Hyperliquid returned an invalid transfer record".to_string();
    let (source, destination, direction, source_chain, destination_chain) = if account_class {
        let to_perp = delta["toPerp"].as_bool().ok_or_else(invalid)?;
        let (source, destination) = if to_perp {
            ("Spot", "Perps")
        } else {
            ("Perps", "Spot")
        };
        (
            account,
            account,
            TransferDirection::Internal,
            source,
            destination,
        )
    } else {
        let source = delta["user"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(invalid)?;
        let destination = delta["destination"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(invalid)?;
        let sent = source.eq_ignore_ascii_case(account);
        let received = destination.eq_ignore_ascii_case(account);
        let direction = match (sent, received) {
            (true, true) => TransferDirection::Internal,
            (true, false) => TransferDirection::Sent,
            (false, true) => TransferDirection::Received,
            // A parent account can receive ledger events for its subaccounts.
            // Do not label an unrelated debit/credit as belonging to this account.
            (false, false) => return Ok(None),
        };
        let chain = if spot { "Spot" } else { "Perps" };
        (source, destination, direction, chain, chain)
    };
    let asset = if spot {
        delta["token"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(invalid)?
    } else {
        "USDC"
    };
    let amount = ledger_number(&delta[if spot { "amount" } else { "usdc" }]).ok_or_else(invalid)?;
    Ok(Some(TransferEntry {
        // Include payload fields because one transaction can emit multiple
        // transfers at the same timestamp. This ID stays runtime-only/redacted.
        id: format!(
            "hl:{}:{}:{kind}:{source}:{destination}:{asset}:{amount}:{source_chain}:{destination_chain}",
            entry.hash, entry.time
        ),
        time: entry.time,
        provider: TransferProvider::Hyperliquid,
        direction,
        asset: asset.to_string(),
        amount,
        source_chain: source_chain.to_string(),
        destination_chain: destination_chain.to_string(),
        source_address: Some(source.to_string()),
        destination_address: Some(destination.to_string()),
        protocol_address: None,
        source_tx: None,
        destination_tx: None,
        ledger_tx: super::super::nonempty(entry.hash),
        status: "Completed".to_string(),
        failed: false,
        fee: ledger_number(&delta["fee"]),
        // Older spot records omit feeToken. Do not assume the transferred
        // token is also the fee token.
        fee_asset: if spot {
            delta["feeToken"]
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .map(str::to_string)
        } else {
            Some("USDC".to_string())
        },
        sweep_fee: None,
        source_confirmations: None,
        destination_confirmations: None,
    }))
}

#[cfg(test)]
mod tests;

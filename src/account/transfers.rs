mod hyperliquid;
mod unit;

#[cfg(test)]
mod tests;

pub(crate) use hyperliquid::fetch_bridge_history;
pub(crate) use unit::fetch_unit_history;

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferProvider {
    Hyperliquid,
    Unit,
}

impl TransferProvider {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Hyperliquid => "Hyperliquid bridge",
            Self::Unit => "Unit (TradeXYZ)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferDirection {
    Deposit,
    Withdrawal,
}

impl TransferDirection {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Deposit => "Deposit",
            Self::Withdrawal => "Withdrawal",
        }
    }
}

/// Runtime-only history. Never include addresses, transaction references, or
/// upstream strings in Debug output (Message logs include these results).
#[derive(Clone)]
pub(crate) struct TransferEntry {
    pub(crate) id: String,
    pub(crate) time: u64,
    pub(crate) provider: TransferProvider,
    pub(crate) direction: TransferDirection,
    pub(crate) asset: String,
    pub(crate) amount: String,
    pub(crate) source_chain: String,
    pub(crate) destination_chain: String,
    pub(crate) source_address: Option<String>,
    pub(crate) destination_address: Option<String>,
    pub(crate) protocol_address: Option<String>,
    pub(crate) source_tx: Option<String>,
    pub(crate) destination_tx: Option<String>,
    pub(crate) ledger_tx: Option<String>,
    pub(crate) status: String,
    pub(crate) failed: bool,
    pub(crate) fee: Option<String>,
    pub(crate) sweep_fee: Option<String>,
    pub(crate) source_confirmations: Option<u64>,
    pub(crate) destination_confirmations: Option<u64>,
}

impl fmt::Debug for TransferEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransferEntry")
            .field("provider", &self.provider)
            .field("direction", &self.direction)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TransferSnapshot {
    pub(crate) entries: Vec<TransferEntry>,
    pub(crate) warning: Option<String>,
    /// Inclusive next start time for native ledger reads. None restarts the
    /// complete history, e.g. after a partial initial read.
    pub(crate) next_start: Option<u64>,
}

fn nonempty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

/// Decimal shifting in strings preserves every satoshi/wei and fractional fee
/// estimate, including amounts larger than the exact range of f64.
fn decimal_units(raw: &str, decimals: usize) -> Option<String> {
    if raw.is_empty() || decimals > 30 || raw.len() > 160 {
        return None;
    }
    let (whole, fraction) = raw.split_once('.').unwrap_or((raw, ""));
    if whole.is_empty()
        || !whole.bytes().all(|c| c.is_ascii_digit())
        || !fraction.bytes().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let padded = format!("{whole:0>width$}{fraction}", width = decimals + 1);
    let point = padded.len() - decimals - fraction.len();
    let integer = padded[..point].trim_start_matches('0');
    let integer = if integer.is_empty() { "0" } else { integer };
    let fraction = padded[point..].trim_end_matches('0');
    Some(if fraction.is_empty() {
        integer.to_string()
    } else {
        format!("{integer}.{fraction}")
    })
}

fn chain_label(chain: &str) -> String {
    match chain {
        "hyperliquid" => "Hyperliquid",
        "bitcoin" => "Bitcoin",
        "ethereum" => "Ethereum",
        "solana" => "Solana",
        "plasma" => "Plasma",
        "monad" => "Monad",
        "zcash" => "Zcash",
        "avalanche" => "Avalanche",
        "base" => "Base",
        other => other,
    }
    .to_string()
}

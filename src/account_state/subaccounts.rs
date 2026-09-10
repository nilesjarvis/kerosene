use crate::api::{API_URL, CLIENT, proxy::HyperliquidRequestExt};
use crate::app_state::TradingTerminal;
use crate::message::RedactedAddress;
use crate::wallet_state::address_book::normalize_wallet_address_value;

use iced::window;
use serde::Deserialize;
use std::collections::HashSet;
use std::fmt;

/// A validated child of the master address used for discovery. Names and
/// addresses are omitted from Debug because messages are routinely logged.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct DiscoveredSubaccount {
    pub(crate) name: String,
    pub(crate) address: RedactedAddress,
    pub(crate) master_address: RedactedAddress,
}

impl fmt::Debug for DiscoveredSubaccount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DiscoveredSubaccount(<redacted>)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AddAccountTarget {
    Master,
    Subaccount(DiscoveredSubaccount),
}

impl fmt::Display for AddAccountTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Master => f.write_str("Master account"),
            Self::Subaccount(account) => write!(
                f,
                "{} · {}",
                account.name,
                TradingTerminal::short_address(&account.address)
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubaccountDiscoveryRequest {
    pub(crate) window_id: window::Id,
    pub(crate) generation: u64,
    pub(crate) master_address: RedactedAddress,
}

#[derive(Clone)]
pub(crate) struct SubaccountDiscoveryResult(pub(crate) Result<Vec<DiscoveredSubaccount>, String>);

impl fmt::Debug for SubaccountDiscoveryResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Ok(accounts) => f.debug_tuple("Subaccounts").field(&accounts.len()).finish(),
            Err(_) => f.write_str("Subaccounts(Error(<redacted>))"),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireSubaccount {
    name: String,
    sub_account_user: String,
    master: String,
}

/// Accept only complete, internally consistent results. Partial results must
/// never let a malformed parent/child relationship become a trading profile.
fn parse_subaccounts(
    requested_master: &str,
    body: &[u8],
) -> Result<Vec<DiscoveredSubaccount>, String> {
    let master = normalize_wallet_address_value(requested_master)
        .ok_or_else(|| "Enter a valid master account address.".to_string())?;
    let records: Vec<WireSubaccount> = serde_json::from_slice(body)
        .map_err(|_| "Hyperliquid returned an invalid subaccount response.".to_string())?;
    let mut addresses = HashSet::new();
    records
        .into_iter()
        .map(|record| {
            let parent = normalize_wallet_address_value(&record.master);
            let address = normalize_wallet_address_value(&record.sub_account_user);
            let Some(address) = address.filter(|address| {
                parent.as_deref() == Some(master.as_str())
                    && address != &master
                    && addresses.insert(address.clone())
            }) else {
                return Err("Hyperliquid returned an invalid subaccount relationship.".to_string());
            };
            let name: String = record
                .name
                .trim()
                .chars()
                .filter(|character| !character.is_control())
                .take(64)
                .collect();
            Ok(DiscoveredSubaccount {
                name: if name.is_empty() {
                    "Subaccount".to_string()
                } else {
                    name
                },
                address: address.into(),
                master_address: master.clone().into(),
            })
        })
        .collect()
}

pub(crate) async fn fetch_subaccounts(master_address: &str) -> SubaccountDiscoveryResult {
    SubaccountDiscoveryResult(fetch_subaccounts_inner(master_address).await)
}

async fn fetch_subaccounts_inner(
    master_address: &str,
) -> Result<Vec<DiscoveredSubaccount>, String> {
    let master = normalize_wallet_address_value(master_address)
        .ok_or_else(|| "Enter a valid master account address.".to_string())?;
    let response = CLIENT
        .post(API_URL)
        .json(&serde_json::json!({"type": "subAccounts", "user": master}))
        .send_info()
        .await
        .map_err(|_| {
            "Could not reach Hyperliquid to discover subaccounts. Try again.".to_string()
        })?;
    if !response.status().is_success() {
        return Err(format!(
            "Subaccount discovery failed (HTTP {}). Try again.",
            response.status().as_u16()
        ));
    }
    let body = response
        .bytes()
        .await
        .map_err(|_| "Could not read Hyperliquid's subaccount response. Try again.".to_string())?;
    parse_subaccounts(&master, &body)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MASTER: &str = "0xabc0000000000000000000000000000000000000";
    const CHILD: &str = "0xdef0000000000000000000000000000000000000";

    fn record(master: &str, child: &str) -> serde_json::Value {
        serde_json::json!({"name": "Strategy", "master": master, "subAccountUser": child})
    }

    fn parse(value: serde_json::Value) -> Result<Vec<DiscoveredSubaccount>, String> {
        parse_subaccounts(MASTER, value.to_string().as_bytes())
    }

    #[test]
    fn validates_and_normalizes_discovered_relationships() {
        let accounts = parse(serde_json::json!([record(
            &MASTER.to_uppercase(),
            &CHILD.to_uppercase()
        )]))
        .expect("valid subaccount response");
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].address.as_str(), CHILD);
        assert_eq!(accounts[0].master_address.as_str(), MASTER);
        assert_eq!(accounts[0].name, "Strategy");
    }

    #[test]
    fn accepts_empty_list_and_rejects_malformed_or_ambiguous_responses() {
        assert!(parse(serde_json::json!([])).expect("empty list").is_empty());
        for invalid in [
            serde_json::Value::Null,
            serde_json::json!({"error": MASTER}),
            serde_json::json!([record(MASTER, "invalid")]),
            serde_json::json!([record("invalid", CHILD)]),
            serde_json::json!([record(CHILD, MASTER)]),
            serde_json::json!([record(MASTER, MASTER)]),
            serde_json::json!([record(MASTER, CHILD), record(MASTER, CHILD)]),
            serde_json::json!([{"name": "missing address", "master": MASTER}]),
        ] {
            let error = parse(invalid).expect_err("invalid response must be rejected");
            assert!(!error.contains(MASTER));
            assert!(!error.contains(CHILD));
        }
        assert!(parse_subaccounts(MASTER, b"not json").is_err());
        assert!(parse_subaccounts("invalid", b"[]").is_err());
    }

    #[test]
    fn message_payload_debug_redacts_names_addresses_and_errors() {
        let mut account = parse(serde_json::json!([record(MASTER, CHILD)]))
            .expect("valid response")
            .remove(0);
        account.name = MASTER.to_string();
        let messages = [
            crate::message::Message::AddAccountTargetSelected(AddAccountTarget::Subaccount(
                account,
            )),
            crate::message::Message::AddAccountSubaccountsLoaded(
                SubaccountDiscoveryRequest {
                    window_id: window::Id::unique(),
                    generation: 1,
                    master_address: MASTER.into(),
                },
                SubaccountDiscoveryResult(Err(CHILD.to_string())),
            ),
        ];
        for message in messages {
            let debug = format!("{message:?}");
            assert!(!debug.contains(MASTER));
            assert!(!debug.contains(CHILD));
        }
    }
}

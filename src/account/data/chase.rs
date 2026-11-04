use crate::account::{OpenOrder, UserFill, normalize_dex_open_order_coins};
use crate::api::{API_URL, CLIENT, proxy::HyperliquidRequestExt};
use crate::config::ReadDataProvider;
use crate::hydromancer_api::HYDROMANCER_API_URL;

use serde::de::DeserializeOwned;
use serde_json::Value;
use std::fmt;
use zeroize::Zeroizing;

/// A complete, single-DEX orders snapshot plus account fills. It must never be
/// installed as a full account snapshot or authorize a replacement placement.
#[derive(Clone)]
pub(crate) struct ChaseRepriceSnapshot {
    pub(crate) open_orders: Vec<OpenOrder>,
    pub(crate) fills: Vec<UserFill>,
}

impl fmt::Debug for ChaseRepriceSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChaseRepriceSnapshot")
            .field("open_orders_len", &self.open_orders.len())
            .field("fills_len", &self.fills.len())
            .finish()
    }
}

fn orders_payload(address: &str, coin: &str) -> (String, Value) {
    let dex = coin.split_once(':').map_or("", |(dex, _)| dex);
    let mut payload = serde_json::json!({"type": "frontendOpenOrders", "user": address});
    if !dex.is_empty() {
        payload["dex"] = Value::String(dex.to_string());
    }
    (dex.to_string(), payload)
}

/// Two required reads run concurrently. Any transport, HTTP, or parse failure
/// rejects the entire snapshot; the lifecycle falls back to full reconciliation.
pub(crate) async fn fetch_chase_reprice_snapshot(
    address: String,
    coin: String,
    provider: ReadDataProvider,
    api_key: Zeroizing<String>,
) -> Result<ChaseRepriceSnapshot, String> {
    let key = (provider == ReadDataProvider::Hydromancer && !api_key.trim().is_empty())
        .then_some(api_key.trim());
    let (dex, orders) = orders_payload(&address, &coin);
    let fills = serde_json::json!({"type": "userFills", "user": address});
    let (orders, fills) = futures::future::join(
        fetch_required_vec::<OpenOrder>("frontendOpenOrders", orders, key),
        fetch_required_vec::<UserFill>("userFills", fills, key),
    )
    .await;
    let mut open_orders = orders?;
    let fills = fills?;
    normalize_dex_open_order_coins(&dex, &mut open_orders);
    Ok(ChaseRepriceSnapshot { open_orders, fills })
}

async fn fetch_required_vec<T: DeserializeOwned>(
    label: &str,
    payload: Value,
    api_key: Option<&str>,
) -> Result<Vec<T>, String> {
    let request = match api_key {
        Some(key) => CLIENT.post(HYDROMANCER_API_URL).bearer_auth(key),
        None => CLIENT.post(API_URL),
    };
    let response = request
        .json(&payload)
        .send_info_for_reconciliation()
        .await?;
    parse_required_response(label, response).await
}

async fn parse_required_response<T: DeserializeOwned>(
    label: &str,
    response: reqwest::Response,
) -> Result<Vec<T>, String> {
    if !response.status().is_success() {
        return Err(format!(
            "{label} reconciliation failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json()
        .await
        .map_err(|_| format!("{label} reconciliation response was invalid"))
}

#[cfg(test)]
mod tests;

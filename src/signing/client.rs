use super::actions::HyperliquidL1Action;
use super::crypto::sign_l1_action;
use super::model::{ExchangeResponse, OrderKind};

use serde_json::Value;
use zeroize::Zeroizing;

const EXCHANGE_URL: &str = "https://api.hyperliquid.xyz/exchange";

fn exchange_nonce_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Single signing entry point for every L1 action. Builds the canonical
/// `{action, nonce, signature, vaultAddress}` payload, posts to /exchange,
/// and parses the response. Adding a new L1 action type now means a new
/// variant on `HyperliquidL1Action` plus a thin wrapper here — no copy of
/// the msgpack-sign-post boilerplate.
async fn sign_and_post(
    private_key: Zeroizing<String>,
    action: &HyperliquidL1Action,
    vault_address: Option<&str>,
) -> Result<ExchangeResponse, String> {
    let msgpack_bytes =
        rmp_serde::to_vec_named(action).map_err(|e| format!("Msgpack error: {e}"))?;
    let nonce = exchange_nonce_ms();
    let signature = sign_l1_action(private_key.as_str(), &msgpack_bytes, vault_address, nonce)?;
    let action_json =
        serde_json::to_value(action).map_err(|e| format!("JSON serialize error: {e}"))?;
    post_exchange(&action_json, &signature, nonce, vault_address).await
}

async fn post_exchange(
    action_json: &Value,
    signature: &Value,
    nonce: u64,
    vault_address: Option<&str>,
) -> Result<ExchangeResponse, String> {
    let payload = serde_json::json!({
        "action": action_json,
        "nonce": nonce,
        "signature": signature,
        "vaultAddress": vault_address,
    });

    let client = crate::api::CLIENT.clone();
    let raw = client
        .post(EXCHANGE_URL)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Exchange request failed: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {e}"))?;

    serde_json::from_str::<ExchangeResponse>(&raw).map_err(|_| format!("Exchange error: {raw}"))
}

/// Place an order on the exchange.
pub async fn place_order(
    private_key: Zeroizing<String>,
    asset: u32,
    is_buy: bool,
    price: String,
    size: String,
    order_kind: OrderKind,
    reduce_only: bool,
) -> Result<ExchangeResponse, String> {
    let action = HyperliquidL1Action::order(asset, is_buy, price, size, order_kind, reduce_only);
    sign_and_post(private_key, &action, None).await
}

/// Cancel an order on the exchange.
pub async fn cancel_order(
    private_key: Zeroizing<String>,
    asset: u32,
    oid: u64,
) -> Result<ExchangeResponse, String> {
    let action = HyperliquidL1Action::cancel(asset, oid);
    sign_and_post(private_key, &action, None).await
}

/// Modify a resting limit order on the exchange.
pub async fn modify_order(
    private_key: Zeroizing<String>,
    oid: u64,
    asset: u32,
    is_buy: bool,
    price: String,
    size: String,
    reduce_only: bool,
) -> Result<ExchangeResponse, String> {
    let action = HyperliquidL1Action::modify(oid, asset, is_buy, price, size, reduce_only);
    sign_and_post(private_key, &action, None).await
}

/// Cancel multiple resting orders in a single signed action.
///
/// Each entry is `(asset_id, oid)`. The wire format is identical to
/// `cancel_order` for a single entry — the exchange accepts 1..N cancels
/// under the same `type: "cancel"` discriminator. Kept module-private until
/// a UI hook lands (e.g. a "Cancel All Open Orders" affordance); the
/// dispatcher path itself is fully exercised by the existing wrappers.
#[allow(dead_code)]
pub(super) async fn batch_cancel(
    private_key: Zeroizing<String>,
    cancels: Vec<(u32, u64)>,
) -> Result<ExchangeResponse, String> {
    let action = HyperliquidL1Action::batch_cancel(cancels);
    sign_and_post(private_key, &action, None).await
}

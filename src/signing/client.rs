use super::actions::{build_cancel_action, build_modify_action, build_order_action};
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

/// Post a signed action to the exchange and parse the response.
async fn post_exchange(
    action_json: &Value,
    signature: &Value,
    nonce: u64,
) -> Result<ExchangeResponse, String> {
    let payload = serde_json::json!({
        "action": action_json,
        "nonce": nonce,
        "signature": signature,
        "vaultAddress": null
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
    let action = build_order_action(asset, is_buy, price, size, order_kind, reduce_only);

    let msgpack_bytes =
        rmp_serde::to_vec_named(&action).map_err(|e| format!("Msgpack error: {e}"))?;
    let nonce = exchange_nonce_ms();

    let signature = sign_l1_action(private_key.as_str(), &msgpack_bytes, None, nonce)?;

    let action_json =
        serde_json::to_value(&action).map_err(|e| format!("JSON serialize error: {e}"))?;

    post_exchange(&action_json, &signature, nonce).await
}

/// Cancel an order on the exchange.
pub async fn cancel_order(
    private_key: Zeroizing<String>,
    asset: u32,
    oid: u64,
) -> Result<ExchangeResponse, String> {
    let action = build_cancel_action(asset, oid);

    let msgpack_bytes =
        rmp_serde::to_vec_named(&action).map_err(|e| format!("Msgpack error: {e}"))?;
    let nonce = exchange_nonce_ms();

    let signature = sign_l1_action(private_key.as_str(), &msgpack_bytes, None, nonce)?;

    let action_json =
        serde_json::to_value(&action).map_err(|e| format!("JSON serialize error: {e}"))?;

    post_exchange(&action_json, &signature, nonce).await
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
    let action = build_modify_action(oid, asset, is_buy, price, size, reduce_only);

    let msgpack_bytes =
        rmp_serde::to_vec_named(&action).map_err(|e| format!("Msgpack error: {e}"))?;
    let nonce = exchange_nonce_ms();

    let signature = sign_l1_action(private_key.as_str(), &msgpack_bytes, None, nonce)?;

    let action_json =
        serde_json::to_value(&action).map_err(|e| format!("JSON serialize error: {e}"))?;

    post_exchange(&action_json, &signature, nonce).await
}

use super::actions::HyperliquidL1Action;
use super::crypto::sign_l1_action;
use super::model::{ExchangeOrderKind, ExchangeResponse};
use crate::app_time::now_ms;
use crate::helpers::sensitive_response_snippet;

use serde_json::Value;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use zeroize::Zeroizing;

const EXCHANGE_URL: &str = "https://api.hyperliquid.xyz/exchange";
const EXCHANGE_EXPIRES_AFTER_MS: u64 = 30_000;
static LAST_EXCHANGE_NONCE_MS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct PlaceOrderRequest {
    pub asset: u32,
    pub is_buy: bool,
    pub price: String,
    pub size: String,
    pub order_kind: ExchangeOrderKind,
    pub reduce_only: bool,
    pub cloid: Option<String>,
}

impl fmt::Debug for PlaceOrderRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlaceOrderRequest")
            .field("asset", &self.asset)
            .field("is_buy", &self.is_buy)
            .field("price", &"<redacted>")
            .field("size", &"<redacted>")
            .field("order_kind", &self.order_kind)
            .field("reduce_only", &self.reduce_only)
            .field("has_cloid", &self.cloid.is_some())
            .finish()
    }
}

fn allocate_exchange_nonce_from(last_nonce_ms: &AtomicU64, now_ms: u64) -> u64 {
    let mut last = last_nonce_ms.load(Ordering::Relaxed);
    loop {
        let next = now_ms.max(last.saturating_add(1));
        match last_nonce_ms.compare_exchange_weak(last, next, Ordering::SeqCst, Ordering::Relaxed) {
            Ok(_) => return next,
            Err(observed) => last = observed,
        }
    }
}

fn allocate_exchange_nonce(now_ms: u64) -> u64 {
    allocate_exchange_nonce_from(&LAST_EXCHANGE_NONCE_MS, now_ms)
}

fn exchange_nonce_ms() -> u64 {
    allocate_exchange_nonce(now_ms())
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
    let payload = build_signed_exchange_payload(private_key, action, vault_address)?;
    post_exchange(&payload).await
}

fn build_signed_exchange_payload(
    private_key: Zeroizing<String>,
    action: &HyperliquidL1Action,
    vault_address: Option<&str>,
) -> Result<Value, String> {
    let nonce = exchange_nonce_ms();
    build_signed_exchange_payload_with_nonce(private_key, action, vault_address, nonce)
}

fn build_signed_exchange_payload_with_nonce(
    private_key: Zeroizing<String>,
    action: &HyperliquidL1Action,
    vault_address: Option<&str>,
    nonce: u64,
) -> Result<Value, String> {
    let msgpack_bytes =
        rmp_serde::to_vec_named(action).map_err(|e| format!("Msgpack error: {e}"))?;
    let expires_after = nonce.saturating_add(EXCHANGE_EXPIRES_AFTER_MS);
    let signature = sign_l1_action(
        private_key.as_str(),
        &msgpack_bytes,
        vault_address,
        nonce,
        Some(expires_after),
    )?;
    drop(private_key);
    let action_json =
        serde_json::to_value(action).map_err(|e| format!("JSON serialize error: {e}"))?;
    Ok(serde_json::json!({
        "action": action_json,
        "nonce": nonce,
        "signature": signature,
        "vaultAddress": vault_address,
        "expiresAfter": expires_after,
    }))
}

async fn post_exchange(payload: &Value) -> Result<ExchangeResponse, String> {
    let client = crate::api::CLIENT.clone();
    let raw = client
        .post(EXCHANGE_URL)
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("Exchange request failed: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {e}"))?;

    parse_exchange_response(&raw)
}

#[cfg(test)]
fn exchange_payload_nonce(payload: &Value) -> Option<u64> {
    payload.get("nonce").and_then(Value::as_u64)
}

#[cfg(test)]
fn exchange_payload_expires_after(payload: &Value) -> Option<u64> {
    payload.get("expiresAfter").and_then(Value::as_u64)
}

#[cfg(test)]
fn exchange_payload_vault_address(payload: &Value) -> Option<&str> {
    payload.get("vaultAddress").and_then(Value::as_str)
}

#[cfg(test)]
fn exchange_payload_signature(payload: &Value) -> Option<&Value> {
    payload.get("signature")
}

#[cfg(test)]
fn exchange_payload_action(payload: &Value) -> Option<&Value> {
    payload.get("action")
}

#[cfg(test)]
fn exchange_payload_contains_private_key(payload: &Value, private_key: &str) -> bool {
    let rendered = payload.to_string();
    rendered.contains(private_key) || rendered.contains(private_key.trim_start_matches("0x"))
}

fn parse_exchange_response(raw: &str) -> Result<ExchangeResponse, String> {
    serde_json::from_str::<ExchangeResponse>(raw)
        .map_err(|_| format!("Exchange error: {}", sensitive_response_snippet(raw)))
}

/// Place an order with a Hyperliquid client order id.
pub async fn place_order_with_cloid(
    private_key: Zeroizing<String>,
    request: PlaceOrderRequest,
) -> Result<ExchangeResponse, String> {
    let action = HyperliquidL1Action::order_with_cloid(
        request.asset,
        request.is_buy,
        request.price,
        request.size,
        request.order_kind,
        request.reduce_only,
        request.cloid,
    );
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

/// Cancel an order by Hyperliquid client order id.
pub async fn cancel_order_by_cloid(
    private_key: Zeroizing<String>,
    asset: u32,
    cloid: String,
) -> Result<ExchangeResponse, String> {
    let action = HyperliquidL1Action::cancel_by_cloid(asset, cloid);
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

/// Update cross or isolated leverage for a perpetual asset.
pub async fn update_leverage(
    private_key: Zeroizing<String>,
    asset: u32,
    is_cross: bool,
    leverage: u32,
) -> Result<ExchangeResponse, String> {
    let action = HyperliquidL1Action::update_leverage(asset, is_cross, leverage);
    sign_and_post(private_key, &action, None).await
}

#[cfg(test)]
mod tests;

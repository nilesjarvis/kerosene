use super::actions::HyperliquidL1Action;
use super::crypto::sign_l1_action;
use super::model::{ExchangeResponse, OrderKind};

use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use zeroize::Zeroizing;

const EXCHANGE_URL: &str = "https://api.hyperliquid.xyz/exchange";
const EXCHANGE_EXPIRES_AFTER_MS: u64 = 30_000;
static LAST_EXCHANGE_NONCE_MS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct PlaceOrderRequest {
    pub asset: u32,
    pub is_buy: bool,
    pub price: String,
    pub size: String,
    pub order_kind: OrderKind,
    pub reduce_only: bool,
    pub cloid: Option<String>,
}

fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
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
    allocate_exchange_nonce(current_time_ms())
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
    let expires_after = nonce.saturating_add(EXCHANGE_EXPIRES_AFTER_MS);
    let signature = sign_l1_action(
        private_key.as_str(),
        &msgpack_bytes,
        vault_address,
        nonce,
        Some(expires_after),
    )?;
    let action_json =
        serde_json::to_value(action).map_err(|e| format!("JSON serialize error: {e}"))?;
    post_exchange(
        &action_json,
        &signature,
        nonce,
        vault_address,
        Some(expires_after),
    )
    .await
}

async fn post_exchange(
    action_json: &Value,
    signature: &Value,
    nonce: u64,
    vault_address: Option<&str>,
    expires_after: Option<u64>,
) -> Result<ExchangeResponse, String> {
    let payload = serde_json::json!({
        "action": action_json,
        "nonce": nonce,
        "signature": signature,
        "vaultAddress": vault_address,
        "expiresAfter": expires_after,
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

#[cfg(test)]
mod tests {
    use super::allocate_exchange_nonce_from;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn exchange_nonce_allocator_is_monotonic_inside_same_millisecond() {
        let last_nonce = AtomicU64::new(0);

        let first = allocate_exchange_nonce_from(&last_nonce, 1_700_000_000_000);
        let second = allocate_exchange_nonce_from(&last_nonce, 1_700_000_000_000);
        let third = allocate_exchange_nonce_from(&last_nonce, 1_700_000_000_000);

        assert_eq!(first, 1_700_000_000_000);
        assert_eq!(second, first + 1);
        assert_eq!(third, second + 1);
    }

    #[test]
    fn exchange_nonce_allocator_never_moves_backwards_when_clock_regresses() {
        let last_nonce = AtomicU64::new(5_000);

        let nonce = allocate_exchange_nonce_from(&last_nonce, 4_000);

        assert_eq!(nonce, 5_001);
        assert_eq!(last_nonce.load(Ordering::SeqCst), 5_001);
    }
}

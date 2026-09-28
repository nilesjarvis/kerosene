use super::{OrderSurface, PreparedExchangeOrder};
use crate::app_time::now_ms;
use crate::signing::ExchangeOrderKind;
use sha3::{Digest, Keccak256};
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};

static LAST_ONE_SHOT_CLOID_NONCE_MS: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// One-Shot Client-Order IDs
// ---------------------------------------------------------------------------

fn allocate_one_shot_cloid_nonce_from(last_nonce_ms: &AtomicU64, now_ms: u64) -> u64 {
    let mut last = last_nonce_ms.load(Ordering::Relaxed);
    loop {
        let next = now_ms.max(last.saturating_add(1));
        match last_nonce_ms.compare_exchange_weak(last, next, Ordering::SeqCst, Ordering::Relaxed) {
            Ok(_) => return next,
            Err(observed) => last = observed,
        }
    }
}

pub(super) fn next_one_shot_place_cloid(
    account_address: &str,
    order: &PreparedExchangeOrder,
) -> String {
    let nonce = allocate_one_shot_cloid_nonce_from(&LAST_ONE_SHOT_CLOID_NONCE_MS, now_ms());
    one_shot_place_cloid(account_address, nonce, order)
}

fn one_shot_place_cloid(
    account_address: &str,
    nonce: u64,
    order: &PreparedExchangeOrder,
) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(b"kerosene:one-shot-place");
    hasher.update(account_address.as_bytes());
    hasher.update(order.surface.cloid_tag().as_bytes());
    hasher.update(order.symbol_key.as_bytes());
    hasher.update(order.asset.to_be_bytes());
    hasher.update([u8::from(order.is_buy)]);
    hasher.update(order.price.as_bytes());
    hasher.update(order.size.as_bytes());
    hasher.update(order.order_kind.cloid_tag().as_bytes());
    hasher.update([u8::from(order.reduce_only)]);
    hasher.update(nonce.to_be_bytes());

    let digest = hasher.finalize();
    let mut cloid = String::with_capacity(34);
    cloid.push_str("0x");
    for byte in digest.iter().take(16) {
        let _ = write!(cloid, "{byte:02x}");
    }
    cloid
}

impl OrderSurface {
    fn cloid_tag(self) -> &'static str {
        match self {
            Self::Ticket => "ticket",
            Self::Preset => "preset",
            Self::QuickOrder => "quick",
            Self::QuickTrade => "quick_trade",
            Self::Hud => "hud",
            Self::ClosePosition => "close",
            Self::Cluster => "cluster",
            Self::ClusterClose => "cluster_close",
            Self::Nuke => "nuke",
            Self::Chase => "chase",
            Self::Twap => "twap",
            Self::Move => "move",
            Self::Cancel => "cancel",
        }
    }
}

impl ExchangeOrderKind {
    fn cloid_tag(self) -> &'static str {
        match self {
            Self::Market => "market",
            Self::Limit => "limit",
            Self::LimitIoc => "limit_ioc",
        }
    }
}

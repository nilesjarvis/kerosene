use super::storage::{envelope_bytes, json_path, load_json};
use super::writer::{CacheWrite, enqueue};
use crate::account_analytics::PortfolioHistory;
use crate::wallet_state::address_book::normalize_wallet_address_value;
use sha3::{Digest, Keccak256};
use std::{fmt, path::Path};

const NAMESPACE: &str = "combined_portfolio_v1";

/// A last-known snapshot, never proof that the portfolio is current. The API
/// returns rolling snapshots, not a timestamp-addressable incremental history.
#[derive(Clone)]
pub(crate) struct CachedPortfolioHistory {
    pub(crate) history: PortfolioHistory,
    pub(crate) fetched_at_ms: u64,
}

impl fmt::Debug for CachedPortfolioHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachedPortfolioHistory")
            .field("buckets", &self.history.buckets.len())
            .finish_non_exhaustive()
    }
}

pub(crate) async fn load(address: String) -> Result<Option<CachedPortfolioHistory>, String> {
    let Some(root) = crate::config::api_cache_dir() else {
        return Ok(None);
    };
    tokio::task::spawn_blocking(move || load_from_dir(&root, &address))
        .await
        .map_err(|_| "Combined portfolio cache read interrupted".to_string())?
}

fn load_from_dir(root: &Path, address: &str) -> Result<Option<CachedPortfolioHistory>, String> {
    let key = wallet_key(address)?;
    let Some(cached) = load_json::<PortfolioHistory>(root, NAMESPACE, &key)? else {
        return Ok(None);
    };
    if cached.fetched_at_ms == 0
        || cached.fetched_at_ms > super::now_ms()
        || !valid_history(&cached.payload)
    {
        return Ok(None);
    }
    Ok(Some(CachedPortfolioHistory {
        history: cached.payload,
        fetched_at_ms: cached.fetched_at_ms,
    }))
}

pub(crate) fn save(
    address: &str,
    history: &PortfolioHistory,
    fetched_at_ms: u64,
) -> Result<(), String> {
    let Some(root) = crate::config::api_cache_dir() else {
        return Ok(());
    };
    let (path, bytes) = snapshot_bytes(&root, address, history, fetched_at_ms)?;
    // Enqueue only after the update handler accepts the request identity. The
    // shared writer orders snapshots/removals and keeps disk I/O off iced.
    enqueue(CacheWrite::SaveBytes { path, bytes });
    Ok(())
}

fn snapshot_bytes(
    root: &Path,
    address: &str,
    history: &PortfolioHistory,
    fetched_at_ms: u64,
) -> Result<(std::path::PathBuf, Vec<u8>), String> {
    if fetched_at_ms == 0 || !valid_history(history) {
        return Err("Invalid combined portfolio snapshot".to_string());
    }
    envelope_bytes(
        root,
        NAMESPACE,
        &wallet_key(address)?,
        fetched_at_ms,
        None,
        history,
    )
}

pub(crate) fn remove(address: &str) -> Result<(), String> {
    let Some(root) = crate::config::api_cache_dir() else {
        return Ok(());
    };
    enqueue(CacheWrite::Remove {
        path: json_path(&root, NAMESPACE, &wallet_key(address)?),
    });
    Ok(())
}

fn wallet_key(address: &str) -> Result<Vec<String>, String> {
    let address = normalize_wallet_address_value(address)
        .ok_or_else(|| "Invalid combined portfolio cache address".to_string())?;
    // Do not expose wallet addresses in filenames or cache-envelope diagnostics.
    Ok(vec![hex::encode(Keccak256::digest(address.as_bytes()))])
}

pub(crate) fn valid_history(history: &PortfolioHistory) -> bool {
    !history.buckets.is_empty()
        && history.buckets.iter().all(|(key, bucket)| {
            matches!(
                key.as_str(),
                "day"
                    | "week"
                    | "month"
                    | "allTime"
                    | "perpDay"
                    | "perpWeek"
                    | "perpMonth"
                    | "perpAllTime"
            ) && bucket
                .account_value_history
                .iter()
                .chain(&bucket.pnl_history)
                .all(|(timestamp, value)| *timestamp > 0 && value.is_finite())
                && bucket.vlm.is_none_or(f64::is_finite)
        })
}

#[cfg(test)]
mod tests;

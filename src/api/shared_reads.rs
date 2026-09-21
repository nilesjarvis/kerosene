//! Share complete public snapshots and in-flight reads across panes and features.
//! Account data is deliberately excluded. Cache hits retain a short, fixed expiry;
//! reading a cached value never extends its lifetime.
use super::proxy::HyperliquidRequestExt;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::Mutex as AsyncMutex;

type Entry<T> = Arc<AsyncMutex<Option<(Instant, Result<T, String>)>>>;

pub(super) struct SharedReads<T> {
    entries: Mutex<HashMap<String, Entry<T>>>,
}

impl<T: Clone> SharedReads<T> {
    pub(super) fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub(super) async fn get(
        &self,
        key: String,
        ttl: Duration,
        fetch: impl Future<Output = Result<T, String>>,
    ) -> Result<T, String> {
        let entry = {
            let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
            if entries.len() >= 256 && !entries.contains_key(&key) {
                // Never evict an in-flight entry: that would duplicate its request.
                entries.retain(|_, entry| Arc::strong_count(entry) > 1);
                if entries.len() >= 256 {
                    return Err("Market data queue busy; retry shortly".to_string());
                }
            }
            entries
                .entry(key)
                .or_insert_with(|| Arc::new(AsyncMutex::new(None)))
                .clone()
        };
        let mut cached = entry.lock().await;
        if let Some((until, value)) = cached.as_ref()
            && *until > Instant::now()
        {
            return value.clone();
        }
        // Cancellation drops this lock. A waiting consumer can then retry.
        let result = fetch.await;
        let lifetime = if result.is_ok() {
            ttl
        } else {
            Duration::from_secs(2)
        };
        *cached = Some((Instant::now() + lifetime, result.clone()));
        result
    }
}

static CONTEXTS: LazyLock<SharedReads<Value>> = LazyLock::new(SharedReads::new);

pub(super) async fn public_info(body: Value) -> Result<Value, String> {
    let kind = body["type"].as_str().unwrap_or("");
    if !matches!(
        kind,
        "metaAndAssetCtxs" | "spotMetaAndAssetCtxs" | "perpDexs"
    ) {
        return Err("Unsupported shared public read".to_string());
    }
    let ttl = Duration::from_secs(if kind == "perpDexs" { 60 } else { 5 });
    CONTEXTS
        .get(body.to_string(), ttl, async {
            super::CLIENT
                .post(super::API_URL)
                .json(&body)
                .send_info()
                .await?
                .error_for_status()
                .map_err(|e| format!("Public market data HTTP error: {e}"))?
                .json()
                .await
                .map_err(|e| format!("Public market data parse failed: {e}"))
        })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[tokio::test]
    async fn thirty_consumers_share_one_read_but_distinct_providers_do_not() {
        let cache = SharedReads::new();
        let count = AtomicUsize::new(0);
        let results = futures::future::join_all((0..30).map(|_| {
            cache.get("hl:BTC:1m".into(), Duration::from_secs(1), async {
                count.fetch_add(1, Ordering::SeqCst);
                tokio::task::yield_now().await;
                Ok(42)
            })
        }))
        .await;
        assert!(results.iter().all(|result| *result == Ok(42)));
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(
            cache
                .get("hydro:BTC:1m".into(), Duration::ZERO, async { Ok(7) })
                .await,
            Ok(7)
        );
        assert_eq!(
            cache
                .get("expired".into(), Duration::ZERO, async { Ok(1) })
                .await,
            Ok(1)
        );
        assert_eq!(
            cache
                .get("expired".into(), Duration::ZERO, async { Ok(2) })
                .await,
            Ok(2)
        );
    }
    #[tokio::test]
    async fn failed_read_does_not_turn_into_a_fresh_success() {
        let cache = SharedReads::<u32>::new();
        assert!(
            cache
                .get("x".into(), Duration::ZERO, async { Err("429".into()) })
                .await
                .is_err()
        );
        assert!(
            cache
                .get("x".into(), Duration::ZERO, async { Ok(1) })
                .await
                .is_err()
        );
    }
}

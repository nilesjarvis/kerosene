use super::{BuilderFee, normalize_builder_address};
use crate::api::proxy::HyperliquidRequestExt;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

const APPROVAL_TTL: Duration = Duration::from_secs(60);
const MAX_APPROVALS: usize = 64;
type ApprovalKey = (String, String);

#[derive(Clone, Copy)]
struct Approval {
    max_fee: u32,
    checked_at: Instant,
}

#[derive(Default)]
struct ApprovalCache {
    entries: HashMap<ApprovalKey, Approval>,
    invalidation_generation: u64,
}

impl ApprovalCache {
    fn get(&self, key: &ApprovalKey, now: Instant) -> Option<u32> {
        self.entries
            .get(key)
            .filter(|value| now.saturating_duration_since(value.checked_at) < APPROVAL_TTL)
            .map(|value| value.max_fee)
    }

    fn insert(&mut self, key: ApprovalKey, max_fee: u32, now: Instant) {
        self.entries
            .retain(|_, value| now.saturating_duration_since(value.checked_at) < APPROVAL_TTL);
        if self.entries.len() >= MAX_APPROVALS
            && !self.entries.contains_key(&key)
            && let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, value)| value.checked_at)
                .map(|(key, _)| key.clone())
        {
            self.entries.remove(&oldest);
        }
        self.entries.insert(
            key,
            Approval {
                max_fee,
                checked_at: now,
            },
        );
    }

    fn invalidate(&mut self, key: &ApprovalKey) -> u64 {
        self.entries.remove(key);
        // The cache is bounded and only one refresh runs at a time, so a global
        // generation avoids an unbounded map of retired account/builder pairs.
        // An unrelated rejection can conservatively require one extra check.
        self.invalidation_generation = self.invalidation_generation.wrapping_add(1);
        self.invalidation_generation
    }

    fn ensure_current(&self, generation: u64) -> Result<(), String> {
        if self.invalidation_generation != generation {
            return Err(
                "Builder approval changed during the check; retry approval in Settings".to_string(),
            );
        }
        Ok(())
    }

    fn insert_if_current(
        &mut self,
        key: ApprovalKey,
        max_fee: u32,
        now: Instant,
        generation: u64,
    ) -> Result<(), String> {
        self.ensure_current(generation)?;
        self.insert(key, max_fee, now);
        Ok(())
    }
}

static APPROVALS: LazyLock<Mutex<ApprovalCache>> =
    LazyLock::new(|| Mutex::new(ApprovalCache::default()));
// Coalesce simultaneous cold-cache placements (e.g. NUKE) before querying /info.
static REFRESH: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn cache_key(account: &str, builder: &BuilderFee) -> Result<ApprovalKey, String> {
    let account = normalize_builder_address(account)
        .map_err(|_| "Builder approval requires a valid trading account".to_string())?;
    Ok((account, builder.address().to_string()))
}

fn cached_max_fee(key: &ApprovalKey) -> Option<u32> {
    APPROVALS.lock().ok()?.get(key, Instant::now())
}

fn validate_max_fee(max_fee: u32, builder: &BuilderFee) -> Result<(), String> {
    if max_fee < builder.fee_tenths_bps() {
        return Err(
            "Builder fee is not approved for this account; approve it in Settings → Integrations"
                .into(),
        );
    }
    Ok(())
}

/// Synchronous admission for multi-wallet batches: every leg must be verified
/// before any leg can be sent. Individual placements recheck at their boundary.
pub(crate) fn cached_order_approval(account: &str, builder: &BuilderFee) -> Result<(), String> {
    let key = cache_key(account, builder)?;
    let max_fee = cached_max_fee(&key).ok_or_else(||
        "Check builder approval for every participating account in Settings → Integrations before trading a wallet cluster".to_string())?;
    validate_max_fee(max_fee, builder)
}

pub(crate) fn invalidate_order_approval(account: &str, builder: &BuilderFee) {
    if let Ok(key) = cache_key(account, builder)
        && let Ok(mut approvals) = APPROVALS.lock()
    {
        approvals.invalidate(&key);
    }
}

fn begin_approval_refresh(key: &ApprovalKey) -> Result<u64, String> {
    APPROVALS
        .lock()
        .map_err(|_| "Builder approval cache unavailable".to_string())
        .map(|mut approvals| approvals.invalidate(key))
}

pub(crate) async fn check_order_approval(
    account: &str,
    builder: &BuilderFee,
) -> Result<(), String> {
    let key = cache_key(account, builder)?;
    if let Some(max_fee) = cached_max_fee(&key) {
        return validate_max_fee(max_fee, builder);
    }
    // A cold approval must not hold an already prepared market order for the
    // full background-read timeout. No order has been signed or posted yet.
    tokio::time::timeout(Duration::from_secs(3), async {
        let _refresh = REFRESH.lock().await;
        if let Some(max_fee) = cached_max_fee(&key) {
            return validate_max_fee(max_fee, builder);
        }
        let generation = begin_approval_refresh(&key)?;
        let max_fee = fetch_builder_approval_inner(key, generation).await?;
        validate_max_fee(max_fee, builder)
    })
    .await
    .map_err(|_| {
        "Builder approval check timed out; check approval in Settings before retrying".to_string()
    })?
}

/// Explicit checks always read the exchange, including after wallet approval.
pub(crate) async fn fetch_builder_approval(
    account: &str,
    builder: &BuilderFee,
) -> Result<u32, String> {
    let key = cache_key(account, builder)?;
    // Invalidate before waiting for another request: even a lock-wait timeout
    // must retire the previous positive approval and any older in-flight read.
    let generation = begin_approval_refresh(&key)?;
    tokio::time::timeout(Duration::from_secs(20), async {
        let _refresh = REFRESH.lock().await;
        fetch_builder_approval_inner(key, generation).await
    })
    .await
    .map_err(|_| "Builder approval check timed out; retry before trading".to_string())?
}

async fn fetch_builder_approval_inner(key: ApprovalKey, generation: u64) -> Result<u32, String> {
    APPROVALS
        .lock()
        .map_err(|_| "Builder approval cache unavailable".to_string())?
        .ensure_current(generation)?;
    let max_fee = tokio::time::timeout(Duration::from_secs(20), async {
        let response = crate::api::CLIENT
            .post(crate::api::API_URL)
            .json(&serde_json::json!({"type":"maxBuilderFee", "user":key.0, "builder":key.1}))
            .send_info()
            .await
            .map_err(|_| "Builder approval check failed; retry before trading".to_string())?;
        if !response.status().is_success() {
            return Err(format!(
                "Builder approval check failed with HTTP {}",
                response.status()
            ));
        }
        let value: serde_json::Value = response
            .json()
            .await
            .map_err(|_| "Invalid builder approval response".to_string())?;
        parse_max_fee(&value)
    })
    .await
    .map_err(|_| "Builder approval check timed out; retry before trading".to_string())??;
    APPROVALS
        .lock()
        .map_err(|_| "Builder approval cache unavailable".to_string())?
        .insert_if_current(key, max_fee, Instant::now(), generation)?;
    Ok(max_fee)
}

fn parse_max_fee(value: &serde_json::Value) -> Result<u32, String> {
    value
        .as_u64()
        .filter(|value| *value <= 1_000)
        .map(|value| value as u32)
        .ok_or_else(|| "Invalid builder approval response".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    const ACCOUNT: &str = "0x1111111111111111111111111111111111111111";
    const BUILDER: &str = "0x2222222222222222222222222222222222222222";

    #[test]
    fn builder_approval_requires_integer_protocol_units() {
        for (input, expected) in [("0", 0), ("1", 1), ("1000", 1000)] {
            assert_eq!(
                parse_max_fee(&serde_json::from_str(input).expect("JSON")),
                Ok(expected)
            );
        }
        for input in ["null", "-1", "1.1", "1001", "\"10\"", "{}"] {
            assert!(parse_max_fee(&serde_json::from_str(input).expect("JSON")).is_err());
        }
        let builder = BuilderFee::new(BUILDER, 10).expect("builder");
        assert!(validate_max_fee(0, &builder).is_err());
        assert!(validate_max_fee(9, &builder).is_err());
        assert!(validate_max_fee(10, &builder).is_ok());
    }

    #[test]
    fn builder_approval_cache_expires_and_is_account_and_builder_scoped() {
        let mut cache = ApprovalCache::default();
        let now = Instant::now();
        let key = (ACCOUNT.to_string(), BUILDER.to_string());
        cache.insert(key.clone(), 10, now);
        assert_eq!(cache.get(&key, now), Some(10));
        assert_eq!(cache.get(&key, now + APPROVAL_TTL), None);
        assert_eq!(
            cache.get(&(BUILDER.to_string(), ACCOUNT.to_string()), now),
            None
        );
        cache.insert(key.clone(), 0, now);
        assert_eq!(cache.get(&key, now), Some(0));
    }

    #[test]
    fn builder_approval_cache_is_bounded() {
        let mut cache = ApprovalCache::default();
        let now = Instant::now();
        for id in 0..MAX_APPROVALS + 1 {
            cache.insert(
                (id.to_string(), BUILDER.to_string()),
                10,
                now + Duration::from_millis(id as u64),
            );
        }
        assert_eq!(cache.entries.len(), MAX_APPROVALS);
        assert!(!cache.entries.contains_key(&("0".into(), BUILDER.into())));
    }

    #[test]
    fn builder_approval_invalidation_rejects_a_stale_inflight_positive_result() {
        let mut cache = ApprovalCache::default();
        let now = Instant::now();
        let key = (ACCOUNT.to_string(), BUILDER.to_string());
        let generation = cache.invalidate(&key);

        // An order rejects while the approval HTTP response is still in flight.
        cache.invalidate(&key);
        assert!(
            cache
                .insert_if_current(key.clone(), 100, now, generation)
                .is_err()
        );
        assert_eq!(cache.get(&key, now), None);

        let next_generation = cache.invalidate(&key);
        cache
            .insert_if_current(key.clone(), 0, now, next_generation)
            .expect("fresh result");
        assert_eq!(cache.get(&key, now), Some(0));
    }

    #[test]
    fn builder_approval_refresh_retires_positive_cache_before_waiting_for_network() {
        let mut cache = ApprovalCache::default();
        let now = Instant::now();
        let key = (ACCOUNT.to_string(), BUILDER.to_string());
        cache.insert(key.clone(), 100, now);
        let previous_generation = cache.invalidation_generation;

        // The request can now time out while waiting for REFRESH; no cached
        // approval or earlier request can resurrect the positive result.
        cache.invalidate(&key);
        assert_eq!(cache.get(&key, now), None);
        assert!(
            cache
                .insert_if_current(key.clone(), 100, now, previous_generation)
                .is_err()
        );
        assert_eq!(cache.get(&key, now), None);
    }

    #[test]
    fn unrelated_builder_invalidation_conservatively_retires_inflight_refresh() {
        let mut cache = ApprovalCache::default();
        let now = Instant::now();
        let key = (ACCOUNT.to_string(), BUILDER.to_string());
        let other_key = (BUILDER.to_string(), ACCOUNT.to_string());
        let generation = cache.invalidate(&key);
        cache.invalidate(&other_key);
        assert!(
            cache
                .insert_if_current(key.clone(), 100, now, generation)
                .is_err()
        );
        assert_eq!(cache.get(&key, now), None);
    }
}

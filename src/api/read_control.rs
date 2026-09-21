//! Process-wide admission for info reads. Exchange writes never enter this queue.
use reqwest::Request;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const WINDOW: Duration = Duration::from_secs(60);
// Leave room for exchange writes and other applications sharing this IP.
const TOTAL_WEIGHT: u32 = 900;
const BACKGROUND_WEIGHT: u32 = 700;

static HYPERLIQUID: LazyLock<ReadGate> = LazyLock::new(ReadGate::new);
static HYDROMANCER: LazyLock<ReadGate> = LazyLock::new(ReadGate::new);

pub(super) struct ReadGate {
    state: Mutex<Budget>,
    background: Arc<Semaphore>,
    critical: Arc<Semaphore>,
}

#[derive(Default)]
struct Budget {
    spent: VecDeque<(Instant, u32, bool)>,
    cooldown: Option<Instant>,
}

impl Budget {
    fn wait(&mut self, now: Instant, weight: u32, critical: bool) -> Option<Duration> {
        while self
            .spent
            .front()
            .is_some_and(|(at, _, _)| now.duration_since(*at) >= WINDOW)
        {
            self.spent.pop_front();
        }
        if let Some(until) = self.cooldown.filter(|until| *until > now) {
            return Some(until.duration_since(now));
        }
        let total: u32 = self.spent.iter().map(|(_, weight, _)| weight).sum();
        let background: u32 = self
            .spent
            .iter()
            .filter(|(_, _, critical)| !critical)
            .map(|(_, weight, _)| weight)
            .sum();
        if total + weight > TOTAL_WEIGHT || (!critical && background + weight > BACKGROUND_WEIGHT) {
            return self
                .spent
                .front()
                .map(|(at, _, _)| (*at + WINDOW).saturating_duration_since(now));
        }
        if weight > 0 {
            self.spent.push_back((now, weight, critical));
        }
        None
    }
}

impl ReadGate {
    fn new() -> Self {
        Self {
            state: Mutex::new(Budget::default()),
            background: Arc::new(Semaphore::new(4)),
            critical: Arc::new(Semaphore::new(2)),
        }
    }

    pub(super) async fn acquire(&self, request: &Request) -> Result<OwnedSemaphorePermit, String> {
        tokio::time::timeout(Duration::from_secs(30), self.acquire_inner(request))
            .await
            .map_err(|_| "Market data read budget busy; retry shortly".to_string())?
    }

    async fn acquire_inner(&self, request: &Request) -> Result<OwnedSemaphorePermit, String> {
        let body = request
            .body()
            .and_then(|body| body.as_bytes())
            .and_then(|bytes| serde_json::from_slice::<Value>(bytes).ok())
            .unwrap_or(Value::Null);
        let kind = body["type"].as_str().unwrap_or("");
        let critical = matches!(
            kind,
            "orderStatus"
                | "clearinghouseState"
                | "spotClearinghouseState"
                | "openOrders"
                | "frontendOpenOrders"
        );
        let weight = if request.url().host_str() == Some("api.hyperliquid.xyz") {
            request_weight(&body)
        } else {
            0
        };
        let semaphore = if critical {
            &self.critical
        } else {
            &self.background
        };
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| "Read queue closed".to_string())?;
        loop {
            let delay = self.state.lock().unwrap_or_else(|e| e.into_inner()).wait(
                Instant::now(),
                weight,
                critical,
            );
            match delay {
                Some(delay) => tokio::time::sleep(delay).await,
                None => return Ok(permit),
            }
        }
    }

    pub(super) fn cool_down(&self, duration: Duration) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let until = Instant::now() + duration;
        state.cooldown = Some(state.cooldown.map_or(until, |old| old.max(until)));
    }
}

pub(super) fn gate(request: &Request) -> Option<&'static ReadGate> {
    if request.method() != reqwest::Method::POST || request.url().path() != "/info" {
        return None;
    }
    match request.url().host_str() {
        Some("api.hyperliquid.xyz") => Some(&HYPERLIQUID),
        Some("api.hydromancer.xyz") => Some(&HYDROMANCER),
        _ => None,
    }
}

fn request_weight(body: &Value) -> u32 {
    match body["type"].as_str().unwrap_or("") {
        "l2Book"
        | "allMids"
        | "clearinghouseState"
        | "orderStatus"
        | "spotClearinghouseState"
        | "exchangeStatus" => 2,
        "userRole" => 60,
        "candleSnapshot" => {
            let req = &body["req"];
            let duration = req["interval"]
                .as_str()
                .and_then(crate::timeframe::Timeframe::from_api_str_opt)
                .map(|tf| tf.duration_ms())
                .unwrap_or(1)
                .max(1);
            let count = req["endTime"]
                .as_u64()
                .unwrap_or(0)
                .saturating_sub(req["startTime"].as_u64().unwrap_or(0))
                / duration
                + 1;
            20 + count.min(5000).div_ceil(60) as u32
        }
        // Conservatively reserve the maximum page cost before dispatch.
        "userFills"
        | "userFillsByTime"
        | "historicalOrders"
        | "userFunding"
        | "userNonFundingLedgerUpdates" => 120,
        _ => 20,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn background_cannot_spend_account_reserve_and_budget_recovers() {
        let now = Instant::now();
        let mut budget = Budget::default();
        for _ in 0..35 {
            assert_eq!(budget.wait(now, 20, false), None);
        }
        assert!(budget.wait(now, 20, false).is_some());
        assert_eq!(budget.wait(now, 2, true), None);
        assert_eq!(budget.wait(now + WINDOW, 20, false), None);
    }
    #[test]
    fn cooldown_blocks_every_priority_without_charging_waiters() {
        let now = Instant::now();
        let mut budget = Budget {
            cooldown: Some(now + WINDOW),
            ..Budget::default()
        };
        assert_eq!(budget.wait(now, 2, true), Some(WINDOW));
        assert!(budget.spent.is_empty());
        assert_eq!(budget.wait(now + WINDOW, 2, true), None);
    }
    #[test]
    fn candle_weight_includes_response_size() {
        assert_eq!(
            request_weight(
                &serde_json::json!({"type":"candleSnapshot", "req":{"interval":"1m","startTime":0,"endTime":299_940_000}})
            ),
            104
        );
    }
}

#[cfg(test)]
mod async_tests {
    use super::*;
    #[tokio::test]
    async fn rate_limit_cooldown_applies_across_distinct_requests() {
        let gate = ReadGate::new();
        gate.cool_down(Duration::from_millis(40));
        let request = reqwest::Client::new()
            .post("https://api.hyperliquid.xyz/info")
            .json(&serde_json::json!({"type":"orderStatus"}))
            .build()
            .expect("request");
        assert!(
            tokio::time::timeout(Duration::from_millis(5), gate.acquire(&request))
                .await
                .is_err()
        );
        assert!(gate.state.lock().expect("state").spent.is_empty());
        assert!(
            tokio::time::timeout(Duration::from_secs(1), gate.acquire(&request))
                .await
                .expect("cooldown expires")
                .is_ok()
        );
    }
    #[test]
    fn exchange_writes_never_enter_the_read_gate() {
        let request = reqwest::Client::new()
            .post("https://api.hyperliquid.xyz/exchange")
            .build()
            .expect("request");
        assert!(gate(&request).is_none());
    }
}

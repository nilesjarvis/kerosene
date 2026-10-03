//! Optional routing for read-only requests to the official Hyperliquid info API.
//! Each proxy owns a connection pool; exchange writes and other services never enter it.

mod response;
mod url;
pub(crate) use url::ProxyUrl;

use super::read_control::{Budget, RequestCost, acquire_proxy_slot};
use reqwest::{Client, Method, Request, RequestBuilder, Response, StatusCode};
use std::sync::{Arc, LazyLock, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::OwnedSemaphorePermit;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const ADMISSION_TIMEOUT: Duration = Duration::from_secs(30);
const FAILURE_COOLDOWN: Duration = Duration::from_secs(15);
const RATE_LIMIT_COOLDOWN: Duration = Duration::from_secs(60);
const MAX_ATTEMPTS: usize = 2;

static POOL: LazyLock<RwLock<Arc<ProxyPool>>> =
    LazyLock::new(|| RwLock::new(Arc::new(ProxyPool::empty(false))));

pub(crate) struct ProxyPool {
    enabled: bool,
    routes: Vec<Client>,
    state: Mutex<PoolState>,
}

#[derive(Default)]
struct PoolState {
    next: usize,
    routes: Vec<RouteState>,
}

#[derive(Default)]
struct RouteState {
    in_flight: usize,
    cooldown_until: Option<Instant>,
    budget: Budget,
}

#[derive(Debug)]
enum AcquireError {
    Unavailable,
    BudgetBusy(Duration),
}

struct RouteLease<'a> {
    pool: &'a ProxyPool,
    index: usize,
}

impl Drop for RouteLease<'_> {
    fn drop(&mut self) {
        let mut state = self.pool.state.lock().unwrap_or_else(|e| e.into_inner());
        state.routes[self.index].in_flight -= 1;
    }
}

impl ProxyPool {
    fn empty(enabled: bool) -> Self {
        Self {
            enabled,
            routes: Vec::new(),
            state: Mutex::new(PoolState::default()),
        }
    }

    pub(crate) fn build(enabled: bool, urls: &[ProxyUrl]) -> Result<Self, String> {
        let mut pool = Self::empty(enabled);
        if enabled {
            for url in urls {
                let url = ProxyUrl::parse(url.as_str())?;
                let proxy = reqwest::Proxy::all(url.as_str())
                    .map_err(|_| "Could not configure proxy".to_string())?;
                let client = super::client_builder()
                    .no_proxy()
                    .proxy(proxy)
                    // A redirect must not send account reads to another destination.
                    .redirect(reqwest::redirect::Policy::none())
                    .build()
                    .map_err(|_| "Could not initialize proxy connection pool".to_string())?;
                pool.routes.push(client);
            }
        }
        pool.state = Mutex::new(PoolState {
            next: 0,
            routes: pool.routes.iter().map(|_| RouteState::default()).collect(),
        });
        Ok(pool)
    }

    fn acquire(
        &self,
        attempted: &[usize],
        now: Instant,
        cost: RequestCost,
    ) -> Result<RouteLease<'_>, AcquireError> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let count = state.routes.len();
        let mut selected = None;
        let mut wait: Option<Duration> = None;
        let mut recovery: Option<Duration> = None;
        for offset in 0..count {
            let index = (state.next + offset) % count;
            let route = &mut state.routes[index];
            if attempted.contains(&index) {
                continue;
            }
            if let Some(until) = route.cooldown_until.filter(|until| *until > now) {
                let delay = until.duration_since(now);
                recovery = Some(recovery.map_or(delay, |previous| previous.min(delay)));
                continue;
            }
            if let Some(delay) = route.budget.delay(now, cost.weight, cost.critical) {
                wait = Some(wait.map_or(delay, |previous| previous.min(delay)));
                continue;
            }
            if selected.is_none_or(|(_, in_flight)| route.in_flight < in_flight) {
                selected = Some((index, route.in_flight));
            }
        }
        let Some((index, _)) = selected else {
            return Err(wait.map_or(AcquireError::Unavailable, |delay| {
                // A cooling route may recover before an exhausted budget does.
                AcquireError::BudgetBusy(recovery.map_or(delay, |recovery| delay.min(recovery)))
            }));
        };
        state.routes[index]
            .budget
            .charge(now, cost.weight, cost.critical);
        state.routes[index].in_flight += 1;
        state.next = (index + 1) % count;
        Ok(RouteLease { pool: self, index })
    }

    async fn admit(
        &self,
        attempted: &[usize],
        cost: RequestCost,
        deadline: Instant,
    ) -> Result<(OwnedSemaphorePermit, RouteLease<'_>), String> {
        let admission = async {
            loop {
                let permit = acquire_proxy_slot(cost).await?;
                let now = Instant::now();
                if now >= deadline {
                    return Err("Hyperliquid proxy admission timed out; retry shortly".to_string());
                }
                match self.acquire(attempted, now, cost) {
                    Ok(route) => return Ok((permit, route)),
                    Err(AcquireError::Unavailable) => {
                        return Err(
                            "Hyperliquid proxies unavailable or cooling down; retry shortly".into(),
                        );
                    }
                    Err(AcquireError::BudgetBusy(delay)) => {
                        // Waiting for an IP budget must not occupy a network slot.
                        drop(permit);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        };
        tokio::time::timeout_at(deadline.into(), admission)
            .await
            .map_err(|_| "Hyperliquid proxy admission timed out; retry shortly".to_string())?
    }

    fn cool_down(&self, index: usize, duration: Duration) {
        let until = Instant::now() + duration;
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let cooldown = &mut state.routes[index].cooldown_until;
        *cooldown = Some(cooldown.map_or(until, |previous| previous.max(until)));
    }

    async fn send(&self, direct: Client, request: Request) -> Result<Response, String> {
        if !self.enabled || !is_official_info(&request) {
            let gate = super::read_control::gate(&request);
            let _permit = match gate {
                Some(gate) => Some(gate.acquire(&request).await?),
                None => None,
            };
            let response = crate::network_activity::execute(&direct, request, false)
                .await
                .map_err(|e| e.to_string())?;
            if response.status() == StatusCode::TOO_MANY_REQUESTS
                && let Some(gate) = gate
            {
                gate.cool_down(retry_after(&response, SystemTime::now()));
            }
            return Ok(response);
        }
        self.send_proxied(request).await
    }

    async fn send_proxied(&self, request: Request) -> Result<Response, String> {
        let timeout = request
            .timeout()
            .copied()
            .unwrap_or(REQUEST_TIMEOUT)
            .min(REQUEST_TIMEOUT);
        if timeout.is_zero() {
            return Err("Hyperliquid proxy request timed out".into());
        }
        let mut deadline = None;
        let admission_deadline = Instant::now() + ADMISSION_TIMEOUT;
        let cost = RequestCost::from_request(&request);
        let mut attempted = Vec::new();
        let mut last_error =
            "Hyperliquid proxies unavailable or cooling down; retry shortly".to_string();
        for _ in 0..MAX_ATTEMPTS {
            let Some(mut attempt) = request.try_clone() else {
                return Err("Hyperliquid read request cannot be replayed".to_string());
            };
            let (_permit, route) = match self
                .admit(&attempted, cost, deadline.unwrap_or(admission_deadline))
                .await
            {
                Ok(admitted) => admitted,
                Err(error) if attempted.is_empty() => return Err(error),
                Err(_) => break,
            };
            let deadline = *deadline.get_or_insert_with(|| Instant::now() + timeout);
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            attempted.push(route.index);
            *attempt.timeout_mut() = Some(remaining);
            let result = tokio::time::timeout_at(deadline.into(), async {
                let response =
                    crate::network_activity::execute(&self.routes[route.index], attempt, true)
                        .await
                        .map_err(|_| {
                            "Hyperliquid proxy connection failed or timed out".to_string()
                        })?;
                if response.status().is_success() {
                    response::buffer(response).await
                } else {
                    Ok(response)
                }
            })
            .await
            .unwrap_or_else(|_| Err("Hyperliquid proxy request timed out".to_string()));
            match result {
                Ok(response) if response.status().is_success() => return Ok(response),
                Ok(response) => {
                    let status = response.status();
                    // Never include proxy response bodies or transport errors: these can
                    // echo Proxy-Authorization, usernames, or the entire proxy URL.
                    last_error =
                        format!("Hyperliquid proxy route returned HTTP {}", status.as_u16());
                    if status == StatusCode::TOO_MANY_REQUESTS {
                        self.cool_down(route.index, retry_after(&response, SystemTime::now()));
                    } else if status.is_server_error()
                        || status == StatusCode::PROXY_AUTHENTICATION_REQUIRED
                    {
                        self.cool_down(route.index, FAILURE_COOLDOWN);
                    } else {
                        return Err(last_error);
                    }
                }
                Err(error) => {
                    self.cool_down(route.index, FAILURE_COOLDOWN);
                    last_error = error;
                }
            }
        }
        Err(last_error)
    }
}

fn is_official_info(request: &Request) -> bool {
    let url = request.url();
    request.method() == Method::POST
        && url.scheme() == "https"
        && url.host_str() == Some("api.hyperliquid.xyz")
        && url.port_or_known_default() == Some(443)
        && url.path() == "/info"
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
}

fn retry_after(response: &Response, now: SystemTime) -> Duration {
    let value = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok());
    let delay = value
        .and_then(|value| {
            value
                .parse::<u64>()
                .ok()
                .map(Duration::from_secs)
                .or_else(|| {
                    chrono::DateTime::parse_from_rfc2822(value)
                        .ok()
                        .and_then(|date| {
                            SystemTime::from(date.with_timezone(&chrono::Utc))
                                .duration_since(now)
                                .ok()
                        })
                })
        })
        .unwrap_or(RATE_LIMIT_COOLDOWN);
    delay.clamp(Duration::from_secs(1), Duration::from_secs(3600))
}

pub(crate) fn install(pool: ProxyPool) {
    // Unit tests build many independent terminals concurrently. Transport tests
    // inject their own pools instead of mutating the production singleton.
    #[cfg(not(test))]
    {
        *POOL.write().unwrap_or_else(|e| e.into_inner()) = Arc::new(pool);
    }
    #[cfg(test)]
    {
        let _ = pool;
    }
}

pub(crate) trait HyperliquidRequestExt {
    fn send_info(self) -> impl Future<Output = Result<Response, String>> + Send;
}

impl HyperliquidRequestExt for RequestBuilder {
    async fn send_info(self) -> Result<Response, String> {
        let (client, request) = self.build_split();
        let request =
            request.map_err(|_| "Could not build Hyperliquid read request".to_string())?;
        let pool = POOL.read().unwrap_or_else(|e| e.into_inner()).clone();
        pool.send(client, request).await
    }
}

#[cfg(test)]
mod tests;

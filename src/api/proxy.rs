//! Optional routing for read-only requests to the official Hyperliquid info API.
//! Each proxy owns a connection pool; exchange writes and other services never enter it.

mod url;
pub(crate) use url::ProxyUrl;

use reqwest::{Client, Method, Request, RequestBuilder, Response, StatusCode};
use std::sync::{Arc, LazyLock, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
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

    fn acquire(&self, attempted: &[usize], now: Instant) -> Option<RouteLease<'_>> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let count = state.routes.len();
        let index = (0..count)
            .map(|offset| (state.next + offset) % count)
            .filter(|index| {
                let route = &state.routes[*index];
                !attempted.contains(index) && route.cooldown_until.is_none_or(|until| until <= now)
            })
            .min_by_key(|index| state.routes[*index].in_flight)?;
        state.routes[index].in_flight += 1;
        state.next = (index + 1) % count;
        Some(RouteLease { pool: self, index })
    }

    fn cool_down(&self, index: usize, duration: Duration) {
        let until = Instant::now() + duration;
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let cooldown = &mut state.routes[index].cooldown_until;
        *cooldown = Some(cooldown.map_or(until, |previous| previous.max(until)));
    }

    async fn send(&self, direct: Client, request: Request) -> Result<Response, String> {
        if !self.enabled || !is_official_info(&request) {
            return crate::network_activity::execute(&direct, request, false)
                .await
                .map_err(|e| e.to_string());
        }
        self.send_proxied(request).await
    }

    async fn send_proxied(&self, request: Request) -> Result<Response, String> {
        let timeout = request
            .timeout()
            .copied()
            .unwrap_or(REQUEST_TIMEOUT)
            .min(REQUEST_TIMEOUT);
        let deadline = Instant::now() + timeout;
        let mut attempted = Vec::new();
        let mut last_error =
            "Hyperliquid proxies unavailable or cooling down; retry shortly".to_string();
        for _ in 0..MAX_ATTEMPTS {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            let Some(route) = self.acquire(&attempted, Instant::now()) else {
                break;
            };
            attempted.push(route.index);
            let Some(mut attempt) = request.try_clone() else {
                return Err("Hyperliquid read request cannot be replayed".to_string());
            };
            *attempt.timeout_mut() = Some(remaining);
            match crate::network_activity::execute(&self.routes[route.index], attempt, true).await {
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
                Err(_) => {
                    self.cool_down(route.index, FAILURE_COOLDOWN);
                    last_error = "Hyperliquid proxy connection failed or timed out".to_string();
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

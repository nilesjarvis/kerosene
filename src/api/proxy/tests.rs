mod balancing;
mod bodies;

use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn urls() -> Vec<ProxyUrl> {
    [
        "http://127.0.0.1:8001",
        "http://127.0.0.1:8002",
        "socks5h://127.0.0.1:8003",
    ]
    .iter()
    .map(|value| ProxyUrl::parse(value).expect("test URL"))
    .collect()
}

fn read_cost() -> RequestCost {
    RequestCost::from_request(&mock_read())
}

#[test]
fn proxy_urls_validate_normalize_and_redact_credentials() {
    for url in [
        "http://proxy.test:8080",
        "https://proxy.test",
        "socks5://proxy.test",
        "socks5h://[::1]:1080",
    ] {
        assert!(ProxyUrl::parse(url).is_ok());
    }
    for url in [
        "",
        "proxy.test:8080",
        "ftp://proxy.test",
        "http://proxy.test/path",
        "http://proxy.test?token=secret",
        "http://proxy.test#secret",
        "http://proxy.test:0",
        "http://proxy.test:99999",
        "http://bad host",
        "http://user:pass@",
        "http://host\\path",
    ] {
        assert!(ProxyUrl::parse(url).is_err(), "invalid test URL accepted");
    }
    assert_eq!(
        ProxyUrl::parse(" http://PROXY.test:80/ "),
        ProxyUrl::parse("http://proxy.test")
    );
    assert_eq!(
        ProxyUrl::parse("socks5://proxy.test"),
        ProxyUrl::parse("socks5://proxy.test:1080")
    );
    let url = ProxyUrl::parse("https://sentinel-user:sentinel-password@proxy.test:8443")
        .expect("authenticated URL");
    assert_eq!(url.label(), "https://proxy.test:8443 (authenticated)");
    assert!(!format!("{url:?}").contains("sentinel"));
    let error = ProxyUrl::parse("http://sentinel-user:sentinel-password@host/path")
        .expect_err("invalid path");
    assert!(!error.contains("sentinel"));
}

#[test]
fn only_official_https_info_posts_are_routed() {
    let client = Client::new();
    assert!(is_official_info(
        &client.post(super::super::API_URL).build().expect("request")
    ));
    for url in [
        "http://api.hyperliquid.xyz/info",
        "https://api.hyperliquid.xyz/exchange",
        "https://api.hyperliquid-testnet.xyz/info",
        "https://api.hydromancer.xyz/info",
        "https://api.hyperliquid.xyz.evil.test/info",
        "https://api.hyperliquid.xyz:444/info",
        "https://api.hyperliquid.xyz/info?x=1",
        "https://api.hyperliquid.xyz/info#x",
    ] {
        assert!(!is_official_info(
            &client.post(url).build().expect("request")
        ));
    }
    assert!(!is_official_info(
        &client.get(super::super::API_URL).build().expect("request")
    ));
}

#[test]
fn concurrent_reads_balance_and_idle_routes_rotate() {
    let pool = ProxyPool::build(true, &urls()).expect("pool");
    let now = Instant::now();
    let a = pool.acquire(&[], now, read_cost()).expect("first route");
    let b = pool.acquire(&[], now, read_cost()).expect("second route");
    let c = pool.acquire(&[], now, read_cost()).expect("third route");
    assert_eq!([a.index, b.index, c.index], [0, 1, 2]);
    drop(b);
    // Route 1 is less busy even though the round-robin cursor points to 0.
    assert_eq!(
        pool.acquire(&[], now, read_cost())
            .expect("idle route")
            .index,
        1
    );
    drop(a);
    drop(c);
    let picked: Vec<_> = (0..6)
        .map(|_| pool.acquire(&[], now, read_cost()).expect("route").index)
        .collect();
    assert_eq!(picked, [2, 0, 1, 2, 0, 1]);
    assert!(
        pool.state
            .lock()
            .expect("state")
            .routes
            .iter()
            .all(|route| route.in_flight == 0)
    );
}

#[test]
fn failed_routes_cool_down_and_reenter_rotation() {
    let pool = ProxyPool::build(true, &urls()).expect("pool");
    pool.cool_down(0, Duration::from_secs(60));
    let before = Instant::now();
    assert!(pool.acquire(&[1, 2], before, read_cost()).is_err());
    assert_eq!(
        pool.acquire(&[1, 2], before + Duration::from_secs(61), read_cost())
            .expect("recovered")
            .index,
        0
    );
    assert!(pool.acquire(&[0, 1, 2], before, read_cost()).is_err());
}

// A loopback HTTP proxy records the real reqwest request and serves an upstream
// response. No public API or external proxy is contacted by these tests.
async fn mock_proxy(
    status: &str,
    headers: &str,
    body: &str,
) -> (ProxyUrl, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let url = ProxyUrl::parse(&format!(
        "http://sentinel-user:sentinel-password@{}",
        listener.local_addr().expect("address")
    ))
    .expect("proxy URL");
    let reply = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",
        body.len()
    );
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let request = read_request(&mut stream).await;
        stream
            .write_all(reply.as_bytes())
            .await
            .expect("write response");
        request
    });
    (url, task)
}

async fn read_request(stream: &mut tokio::net::TcpStream) -> String {
    let mut request = Vec::new();
    loop {
        let mut bytes = [0; 1024];
        let n = stream.read(&mut bytes).await.expect("read");
        if n == 0 {
            break;
        }
        request.extend_from_slice(&bytes[..n]);
        if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
            let header = String::from_utf8_lossy(&request[..end]).to_lowercase();
            let length: usize = header
                .lines()
                .find_map(|line| {
                    line.strip_prefix("content-length: ")
                        .and_then(|length| length.parse().ok())
                })
                .unwrap_or(0);
            if request.len() >= end + 4 + length {
                break;
            }
        }
    }
    String::from_utf8(request).expect("HTTP request")
}

fn mock_read() -> Request {
    Client::new()
        .post("http://hyperliquid.test/info")
        .json(&serde_json::json!({"type": "allMids"}))
        .build()
        .expect("mock request")
}

#[tokio::test]
async fn proxy_forwards_body_and_authentication() {
    let (url, server) = mock_proxy("200 OK", "", r#"{"BTC":"100"}"#).await;
    let pool = ProxyPool::build(true, &[url]).expect("pool");
    let response = pool
        .send_proxied(mock_read())
        .await
        .expect("proxy response");
    assert_eq!(response.text().await.expect("body"), r#"{"BTC":"100"}"#);
    let request = server.await.expect("server");
    assert!(request.starts_with("POST http://hyperliquid.test/info HTTP/1.1"));
    assert!(
        request
            .to_lowercase()
            .contains("proxy-authorization: basic ")
    );
    assert!(request.contains(r#"{"type":"allMids"}"#));
}

#[tokio::test]
async fn rate_limited_route_retries_another_proxy_and_honors_cooldown() {
    let (limited, limited_server) = mock_proxy(
        "429 Too Many Requests",
        "Retry-After: 120\r\n",
        "sentinel-password",
    )
    .await;
    let (healthy, healthy_server) = mock_proxy("200 OK", "", "{}").await;
    let pool = ProxyPool::build(true, &[limited, healthy]).expect("pool");
    assert!(
        pool.send_proxied(mock_read())
            .await
            .expect("failover response")
            .status()
            .is_success()
    );
    limited_server.await.expect("limited proxy");
    healthy_server.await.expect("healthy proxy");
    let until = pool.state.lock().expect("state").routes[0]
        .cooldown_until
        .expect("cooldown");
    assert!(until > Instant::now() + Duration::from_secs(115));
    assert!(pool.acquire(&[1], Instant::now(), read_cost()).is_err());
}

#[tokio::test]
async fn failure_retries_are_bounded_and_errors_never_echo_proxy_secrets() {
    let (a, sa) = mock_proxy("502 Bad Gateway", "", "sentinel-user sentinel-password").await;
    let (b, sb) = mock_proxy("407 Proxy Authentication Required", "", "sentinel-password").await;
    let pool = ProxyPool::build(true, &[a, b, urls()[0].clone()]).expect("pool");
    let error = pool
        .send_proxied(mock_read())
        .await
        .expect_err("two failures");
    assert!(error.contains("407"));
    assert!(!error.contains("sentinel"));
    sa.await.expect("proxy a");
    sb.await.expect("proxy b");
    assert!(
        pool.state.lock().expect("state").routes[2]
            .cooldown_until
            .is_none()
    );
}

#[tokio::test]
async fn https_uses_connect_and_redacts_failed_tunnel_errors() {
    let (proxy, server) =
        mock_proxy("407 Proxy Authentication Required", "", "sentinel-password").await;
    let pool = ProxyPool::build(true, &[proxy]).expect("pool");
    let client = Client::new();
    let error = pool
        .send(
            client.clone(),
            client
                .post(super::super::API_URL)
                .json(&serde_json::json!({"type":"allMids"}))
                .build()
                .expect("request"),
        )
        .await
        .expect_err("CONNECT failure");
    assert!(!error.contains("sentinel"));
    assert!(
        server
            .await
            .expect("proxy")
            .starts_with("CONNECT api.hyperliquid.xyz:443 HTTP/1.1")
    );
}

#[tokio::test]
async fn non_official_requests_bypass_even_an_empty_enabled_pool() {
    let (url, server) = mock_proxy("200 OK", "", "{}").await;
    let mut target = reqwest::Url::parse(url.as_str()).expect("target");
    target.set_username("").expect("remove user");
    target.set_password(None).expect("remove password");
    let direct = Client::builder().no_proxy().build().expect("client");
    let response = ProxyPool::empty(true)
        .send(
            direct.clone(),
            direct.post(target).build().expect("request"),
        )
        .await
        .expect("direct response");
    assert!(response.status().is_success());
    assert!(server.await.expect("server").starts_with("POST / HTTP/1.1"));
}

#[tokio::test]
async fn unavailable_pool_never_falls_back_to_direct() {
    let direct = Client::new();
    let request = direct.post(super::super::API_URL).build().expect("request");
    let result = tokio::time::timeout(
        Duration::from_millis(100),
        ProxyPool::empty(true).send(direct, request),
    )
    .await
    .expect("immediate failure");
    assert!(result.expect_err("empty pool").contains("unavailable"));
}

#[tokio::test]
async fn cancellation_releases_the_route_and_request_timeout_is_bounded() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let url = ProxyUrl::parse(&format!(
        "http://{}",
        listener.local_addr().expect("address")
    ))
    .expect("URL");
    let pool = ProxyPool::build(true, &[url]).expect("pool");
    assert!(
        tokio::time::timeout(Duration::from_millis(50), pool.send_proxied(mock_read()))
            .await
            .is_err()
    );
    assert_eq!(pool.state.lock().expect("state").routes[0].in_flight, 0);
    let mut request = mock_read();
    *request.timeout_mut() = Some(Duration::from_millis(50));
    let error = tokio::time::timeout(Duration::from_secs(1), pool.send_proxied(request))
        .await
        .expect("bounded timeout")
        .expect_err("no reply");
    assert!(error.contains("timed out"));
    assert_eq!(pool.state.lock().expect("state").routes[0].in_flight, 0);
}

#[tokio::test]
async fn retry_after_accepts_http_dates_and_defaults_when_invalid() {
    for (header, expected) in [
        ("Retry-After: invalid\r\n", 60),
        ("Retry-After: 0\r\n", 1),
        ("Retry-After: 999999\r\n", 3600),
    ] {
        let (proxy, server) = mock_proxy("429 Too Many Requests", header, "").await;
        let response = Client::builder()
            .proxy(reqwest::Proxy::all(proxy.as_str()).expect("proxy"))
            .build()
            .expect("client")
            .execute(mock_read())
            .await
            .expect("response");
        assert_eq!(
            retry_after(&response, SystemTime::now()),
            Duration::from_secs(expected)
        );
        server.await.expect("server");
    }
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let date = chrono::DateTime::<chrono::Utc>::from(now + Duration::from_secs(120)).to_rfc2822();
    let (proxy, server) = mock_proxy(
        "429 Too Many Requests",
        &format!("Retry-After: {date}\r\n"),
        "",
    )
    .await;
    let response = Client::builder()
        .proxy(reqwest::Proxy::all(proxy.as_str()).expect("proxy"))
        .build()
        .expect("client")
        .execute(mock_read())
        .await
        .expect("response");
    assert_eq!(retry_after(&response, now), Duration::from_secs(120));
    server.await.expect("server");
}

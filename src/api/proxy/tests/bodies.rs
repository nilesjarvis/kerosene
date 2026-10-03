use super::*;
use tokio::sync::oneshot;

struct DelayedProxy {
    url: ProxyUrl,
    headers_sent: oneshot::Receiver<()>,
    release_body: oneshot::Sender<()>,
    server: tokio::task::JoinHandle<()>,
}

async fn delayed_body_proxy() -> DelayedProxy {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let url = ProxyUrl::parse(&format!(
        "http://{}",
        listener.local_addr().expect("address")
    ))
    .expect("URL");
    let (headers_tx, headers_sent) = oneshot::channel();
    let (release_body, body_rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        read_request(&mut stream).await;
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n",
            )
            .await
            .expect("headers");
        let _ = headers_tx.send(());
        if body_rx.await.is_ok() {
            let _ = stream.write_all(b"{}").await;
        }
    });
    DelayedProxy {
        url,
        headers_sent,
        release_body,
        server,
    }
}

#[tokio::test]
async fn slow_body_retains_load_until_complete_and_preserves_response_metadata() {
    let mut proxy = delayed_body_proxy().await;
    let pool = ProxyPool::build(true, &[proxy.url, urls()[0].clone()]).expect("pool");
    let mut send = Box::pin(pool.send_proxied(mock_read()));
    tokio::select! {
        headers = &mut proxy.headers_sent => headers.expect("headers sent"),
        _ = &mut send => panic!("read completed before its body"),
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut send)
            .await
            .is_err()
    );
    assert_eq!(pool.state.lock().expect("state").routes[0].in_flight, 1);
    assert_eq!(
        pool.acquire(&[], Instant::now(), read_cost())
            .expect("idle route")
            .index,
        1
    );
    proxy.release_body.send(()).expect("release body");
    let response = send.await.expect("complete response");
    assert_eq!(pool.state.lock().expect("state").routes[0].in_flight, 0);
    assert_eq!(response.url().as_str(), "http://hyperliquid.test/info");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.version(), reqwest::Version::HTTP_11);
    assert_eq!(response.headers()["content-type"], "application/json");
    assert_eq!(
        response.json::<serde_json::Value>().await.expect("JSON"),
        serde_json::json!({})
    );
    proxy.server.await.expect("server");
}

#[tokio::test]
async fn cancelling_a_body_releases_the_route_without_marking_it_failed() {
    let mut proxy = delayed_body_proxy().await;
    let pool = ProxyPool::build(true, &[proxy.url]).expect("pool");
    let mut send = Box::pin(pool.send_proxied(mock_read()));
    tokio::select! {
        headers = &mut proxy.headers_sent => headers.expect("headers sent"),
        _ = &mut send => panic!("read completed before its body"),
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut send)
            .await
            .is_err()
    );
    assert_eq!(pool.state.lock().expect("state").routes[0].in_flight, 1);
    drop(send);
    {
        let state = pool.state.lock().expect("state");
        assert_eq!(state.routes[0].in_flight, 0);
        assert!(state.routes[0].cooldown_until.is_none());
    }
    drop(proxy.release_body);
    proxy.server.await.expect("server");
}

async fn raw_proxy(reply: &'static [u8]) -> (ProxyUrl, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let url = ProxyUrl::parse(&format!(
        "http://{}",
        listener.local_addr().expect("address")
    ))
    .expect("URL");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        read_request(&mut stream).await;
        stream.write_all(reply).await.expect("response");
    });
    (url, server)
}

#[tokio::test]
async fn truncated_and_oversized_bodies_cool_down_and_retry_another_route() {
    for reply in [
        &b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nsentinel-password"[..],
        &b"HTTP/1.1 200 OK\r\nContent-Length: 16777217\r\n\r\n"[..],
    ] {
        let (broken, broken_server) = raw_proxy(reply).await;
        let (healthy, healthy_server) = mock_proxy("200 OK", "", r#"{"ok":true}"#).await;
        let pool = ProxyPool::build(true, &[broken, healthy]).expect("pool");
        let response = pool.send_proxied(mock_read()).await.expect("failover");
        assert_eq!(response.text().await.expect("body"), r#"{"ok":true}"#);
        {
            let state = pool.state.lock().expect("state");
            assert!(state.routes[0].cooldown_until.is_some());
            assert!(state.routes[1].cooldown_until.is_none());
            assert!(state.routes.iter().all(|route| route.in_flight == 0));
        }
        broken_server.await.expect("broken server");
        healthy_server.await.expect("healthy server");
    }
}

#[tokio::test]
async fn body_failures_are_redacted_when_no_other_route_exists() {
    let (broken, server) =
        raw_proxy(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nsentinel-user sentinel-password")
            .await;
    let pool = ProxyPool::build(true, &[broken]).expect("pool");
    let error = pool
        .send_proxied(mock_read())
        .await
        .expect_err("broken body");
    assert!(error.contains("body failed"));
    assert!(!error.contains("sentinel"));
    server.await.expect("server");
}

#[tokio::test]
async fn stalled_body_obeys_request_timeout_and_cools_down() {
    let proxy = delayed_body_proxy().await;
    let pool = ProxyPool::build(true, &[proxy.url]).expect("pool");
    let mut request = mock_read();
    *request.timeout_mut() = Some(Duration::from_millis(40));
    let error = tokio::time::timeout(Duration::from_secs(2), pool.send_proxied(request))
        .await
        .expect("bounded body timeout")
        .expect_err("stalled body");
    assert!(error.contains("timed out"));
    {
        let state = pool.state.lock().expect("state");
        assert_eq!(state.routes[0].in_flight, 0);
        assert!(state.routes[0].cooldown_until.is_some());
    }
    drop(proxy.release_body);
    proxy.server.await.expect("server");
}

#[tokio::test]
async fn retry_admission_cannot_outlive_the_request_deadline() {
    let (failing, server) = mock_proxy("503 Service Unavailable", "", "").await;
    let pool = ProxyPool::build(true, &[failing, urls()[0].clone()]).expect("pool");
    // Exhaust the alternate route's background budget before the first fails.
    for _ in 0..350 {
        drop(
            pool.acquire(&[0], Instant::now(), read_cost())
                .expect("capacity"),
        );
    }
    let mut request = mock_read();
    *request.timeout_mut() = Some(Duration::from_millis(40));
    let error = tokio::time::timeout(Duration::from_secs(2), pool.send_proxied(request))
        .await
        .expect("retry wait respects request deadline")
        .expect_err("alternate route exhausted");
    assert!(error.contains("503"));
    server.await.expect("server");
}

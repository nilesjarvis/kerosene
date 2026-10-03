use super::*;

fn background_cost() -> RequestCost {
    RequestCost {
        weight: 20,
        critical: false,
    }
}

#[test]
fn each_route_has_its_own_budget_and_account_reserve() {
    let pool = ProxyPool::build(true, &urls()).expect("pool");
    let now = Instant::now();
    let mut counts = [0; 3];
    for _ in 0..105 {
        let route = pool.acquire(&[], now, background_cost()).expect("capacity");
        counts[route.index] += 1;
    }
    assert_eq!(counts, [35, 35, 35]);
    assert!(matches!(
        pool.acquire(&[], now, background_cost()),
        Err(AcquireError::BudgetBusy(_))
    ));
    let critical = RequestCost {
        weight: 2,
        critical: true,
    };
    for _ in 0..300 {
        drop(pool.acquire(&[], now, critical).expect("account reserve"));
    }
    assert!(matches!(
        pool.acquire(&[], now, critical),
        Err(AcquireError::BudgetBusy(_))
    ));
    assert!(
        pool.acquire(&[], now + Duration::from_secs(60), background_cost())
            .is_ok()
    );
}

#[test]
fn exhausted_routes_are_skipped_and_unavailable_routes_spend_nothing() {
    let pool = ProxyPool::build(true, &urls()).expect("pool");
    pool.cool_down(1, Duration::from_secs(15));
    pool.cool_down(2, Duration::from_secs(15));
    let now = Instant::now();
    for _ in 0..100 {
        assert!(matches!(
            pool.acquire(&[0], now, background_cost()),
            Err(AcquireError::Unavailable)
        ));
    }
    for _ in 0..35 {
        assert_eq!(
            pool.acquire(&[], now, background_cost())
                .expect("route")
                .index,
            0
        );
    }
    assert!(matches!(
        pool.acquire(&[], now, background_cost()),
        Err(AcquireError::BudgetBusy(_))
    ));
    // Once cooldown ends, both unused budgets are intact. Route 0 is still full.
    let later = now + Duration::from_secs(16);
    let mut counts = [0; 3];
    for _ in 0..70 {
        let route = pool
            .acquire(&[], later, background_cost())
            .expect("capacity");
        counts[route.index] += 1;
    }
    assert_eq!(counts, [0, 35, 35]);
}

#[tokio::test]
async fn real_sends_distribute_beyond_a_single_ip_budget() {
    let mut proxies = Vec::new();
    let mut servers = Vec::new();
    for _ in 0..3 {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        proxies.push(
            ProxyUrl::parse(&format!(
                "http://{}",
                listener.local_addr().expect("address")
            ))
            .expect("URL"),
        );
        servers.push(tokio::spawn(async move {
            for _ in 0..12 {
                let (mut stream, _) = listener.accept().await.expect("accept");
                assert!(read_request(&mut stream).await.contains(r#""type":"meta""#));
                stream
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                    )
                    .await
                    .expect("response");
            }
        }));
    }
    let pool = ProxyPool::build(true, &proxies).expect("pool");
    // This host also exercises the direct gate in the old implementation.
    // All connections terminate at loopback proxies; no public API is contacted.
    for _ in 0..36 {
        let request = Client::new()
            .post("http://api.hyperliquid.xyz/info")
            .json(&serde_json::json!({"type":"meta"}))
            .build()
            .expect("request");
        let response = tokio::time::timeout(Duration::from_secs(2), pool.send_proxied(request))
            .await
            .expect("route capacity available")
            .expect("response");
        assert_eq!(response.text().await.expect("body"), "{}");
    }
    for server in servers {
        tokio::time::timeout(Duration::from_secs(2), server)
            .await
            .expect("each route received twelve requests")
            .expect("server");
    }
}

#[test]
fn budget_wait_rechecks_a_route_that_recovers_sooner() {
    let pool = ProxyPool::build(true, &urls()[..2]).expect("pool");
    pool.cool_down(1, Duration::from_secs(15));
    let now = Instant::now();
    for _ in 0..35 {
        drop(pool.acquire(&[], now, background_cost()).expect("capacity"));
    }
    assert!(matches!(
        pool.acquire(&[], now, background_cost()),
        Err(AcquireError::BudgetBusy(delay)) if delay <= Duration::from_secs(15)
    ));
    assert_eq!(
        pool.acquire(&[], now + Duration::from_secs(16), background_cost())
            .expect("recovered route")
            .index,
        1
    );
}

#[tokio::test]
async fn unavailable_sends_do_not_exhaust_admission() {
    let client = Client::new();
    let pool = ProxyPool::empty(true);
    for _ in 0..40 {
        let request = client
            .post(super::super::super::API_URL)
            .json(&serde_json::json!({"type":"meta"}))
            .build()
            .expect("request");
        let error =
            tokio::time::timeout(Duration::from_secs(2), pool.send(client.clone(), request))
                .await
                .expect("no budget wait")
                .expect_err("empty pool");
        assert!(error.contains("unavailable"));
    }
}

#[tokio::test]
async fn admission_timeout_does_not_charge_or_lease_a_route() {
    let pool = ProxyPool::build(true, &urls()[..1]).expect("pool");
    let now = Instant::now();
    for _ in 0..35 {
        drop(pool.acquire(&[], now, background_cost()).expect("capacity"));
    }
    let admitted = pool
        .admit(&[], background_cost(), now + Duration::from_millis(30))
        .await;
    assert!(matches!(admitted, Err(error) if error.contains("timed out")));
    let later = now + Duration::from_secs(60);
    for _ in 0..35 {
        drop(
            pool.acquire(&[], later, background_cost())
                .expect("full recovered budget"),
        );
    }
    assert_eq!(pool.state.lock().expect("state").routes[0].in_flight, 0);
}

#[tokio::test]
async fn unreplayable_requests_and_zero_timeouts_do_not_spend_capacity() {
    let pool = ProxyPool::build(true, &urls()[..1]).expect("pool");
    let body =
        reqwest::Body::wrap_stream(futures::stream::empty::<Result<Vec<u8>, std::io::Error>>());
    let request = Client::new()
        .post("http://hyperliquid.test/info")
        .body(body)
        .build()
        .expect("request");
    assert!(
        pool.send_proxied(request)
            .await
            .expect_err("streaming request")
            .contains("replayed")
    );
    let mut request = mock_read();
    *request.timeout_mut() = Some(Duration::ZERO);
    assert!(
        pool.send_proxied(request)
            .await
            .expect_err("expired request")
            .contains("timed out")
    );
    for _ in 0..35 {
        drop(
            pool.acquire(&[], Instant::now(), background_cost())
                .expect("full budget"),
        );
    }
}

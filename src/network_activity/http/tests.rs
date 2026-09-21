use super::*;

#[test]
fn http_metadata_retains_only_allowlisted_operation_and_provider() {
    let client = Client::new();
    let request = client
        .post("https://name:password@api.hyperliquid.xyz/info?token=PRIVATE")
        .bearer_auth("PRIVATE-KEY")
        .json(&serde_json::json!({"type":"userFills", "user":"PRIVATE-WALLET"}))
        .build()
        .expect("request");
    let entry = request_metadata(&request, true);
    assert_eq!(entry.provider, Provider::Hyperliquid);
    assert_eq!(entry.operation, "userFills");
    assert_eq!(entry.method, "POST");
    assert!(entry.proxied);
    let debug = format!("{entry:?}");
    for secret in ["PRIVATE", "password", "token", "name", "https:"] {
        assert!(!debug.contains(secret));
    }

    let exchange = client.post("https://api.hyperliquid.xyz/exchange")
        .json(&serde_json::json!({"action":{"type":"order","orders":[{"p":"PRIVATE"}]},"signature":"PRIVATE"})).build().expect("request");
    assert_eq!(request_metadata(&exchange, false).operation, "order");
    assert!(!format!("{:?}", request_metadata(&exchange, false)).contains("PRIVATE"));
}

#[test]
fn private_paths_hosts_and_unknown_payloads_are_not_retained() {
    for url in [
        "https://private.example/PRIVATE?token=PRIVATE",
        "https://api.hyperliquid.xyz/PRIVATE",
        "https://api.hyperliquid.xyz.evil.test/info",
    ] {
        let request = Client::new()
            .get(url)
            .json(&serde_json::json!({"type":"PRIVATE"}))
            .build()
            .expect("request");
        let entry = request_metadata(&request, false);
        assert_eq!(entry.operation, "request");
        assert!(!format!("{entry:?}").contains("PRIVATE"));
    }
}

#[test]
fn graphql_operations_are_named_without_retaining_query_variables() {
    let request = Client::new()
        .post("https://api.hyperdash.com/graphql")
        .json(&serde_json::json!({"operationName":"GetTickerPositions", "variables":{"wallet":"PRIVATE"}}))
        .build().expect("request");
    let entry = request_metadata(&request, false);
    assert_eq!(entry.provider, Provider::Hyperdash);
    assert_eq!(entry.operation, "GetTickerPositions");
    assert!(!format!("{entry:?}").contains("PRIVATE"));
}

#[test]
fn cancelled_attempts_keep_their_request_id_and_finish_once() {
    let request = Client::new()
        .get("http://localhost/")
        .build()
        .expect("request");
    let pending = PendingRequest::start(&request, false);
    let id = pending.entry.request_id;
    drop(pending);
    let snapshot = crate::network_activity::snapshot();
    let entries: Vec<_> = snapshot
        .entries
        .iter()
        .filter(|entry| entry.request_id == id)
        .collect();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].kind, ActivityKind::HttpSend);
    assert_eq!(entries[1].kind, ActivityKind::HttpCancelled);
}

#[tokio::test]
async fn observation_preserves_request_body_response_status_and_response_body() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("loopback listener");
    let address = listener.local_addr().expect("address");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("connection");
        let mut bytes = Vec::new();
        loop {
            let mut buf = [0; 1024];
            let count = stream.read(&mut buf).await.expect("read request");
            assert!(count > 0);
            bytes.extend_from_slice(&buf[..count]);
            if bytes.windows(7).any(|part| part == b"PAYLOAD") {
                break;
            }
        }
        stream.write_all(b"HTTP/1.1 429 Too Many Requests\r\nContent-Length: 4\r\nConnection: close\r\n\r\nBODY").await.expect("response");
        bytes
    });
    let response = Client::builder()
        .no_proxy()
        .build()
        .expect("client")
        .post(format!("http://{address}/info"))
        .body("PAYLOAD")
        .send_observed()
        .await
        .expect("response");
    assert_eq!(response.status().as_u16(), 429);
    assert_eq!(response.text().await.expect("response body"), "BODY");
    let request = server.await.expect("server");
    assert!(request.starts_with(b"POST /info HTTP/1.1"));
}

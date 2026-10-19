use super::*;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

const EMPTY_CATALOG: &str =
    r#"{"schemaVersion":1,"indexedThroughBlock":1234,"tradingPaused":false,"instruments":[]}"#;

struct MockServer {
    url: String,
    ready: oneshot::Receiver<()>,
    release: Option<oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<()>,
}

impl MockServer {
    async fn start(response: Vec<u8>, keep_open: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock listener");
        let url = format!(
            "http://{}/state/trading?token=private-sentinel",
            listener.local_addr().expect("mock address")
        );
        let (ready_tx, ready) = oneshot::channel();
        let (release, release_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("mock connection");
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                assert!(request.len() < 8192, "bounded request headers");
                request.push(socket.read_u8().await.expect("request byte"));
            }
            let request = String::from_utf8(request).expect("ASCII request");
            assert!(request.starts_with("GET /state/trading?token=private-sentinel HTTP/1.1\r\n"));
            assert!(!request.to_ascii_lowercase().contains("authorization:"));
            // Oversized responses may be rejected before the server finishes writing.
            let _ = socket.write_all(&response).await;
            let _ = ready_tx.send(());
            if keep_open {
                let _ = release_rx.await;
            }
        });
        Self {
            url,
            ready,
            release: Some(release),
            task,
        }
    }

    async fn finish(&mut self) {
        self.release.take();
        tokio::time::timeout(Duration::from_secs(3), &mut self.task)
            .await
            .expect("mock server deadline")
            .expect("mock server task");
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn client(timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(timeout)
        .build()
        .expect("mock client")
}

fn response(status: u16, body: &[u8], length: usize) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
    ).into_bytes();
    response.extend_from_slice(body);
    response
}

#[tokio::test]
async fn papertrade_http_errors_report_status_without_response_or_url() {
    let client = client(Duration::from_secs(2));
    for status in [401, 403, 429, 500, 503] {
        let body = b"private-sentinel";
        let mut server = MockServer::start(response(status, body, body.len()), false).await;
        let error = fetch_markets_request(client.get(&server.url))
            .await
            .expect_err("HTTP error");
        server.finish().await;
        assert_eq!(
            error,
            format!("Papertrade markets unavailable (HTTP {status}).")
        );
        assert!(!error.contains("private-sentinel"));
        assert!(!error.contains(&server.url));
    }
}

#[tokio::test]
async fn papertrade_http_body_limit_accepts_boundary_and_rejects_streamed_overflow() {
    let client = client(Duration::from_secs(3));
    for length in [MAX_RESPONSE_BYTES, MAX_RESPONSE_BYTES + 1] {
        let mut body = EMPTY_CATALOG.as_bytes().to_vec();
        body.resize(length, b' ');
        // Chunked transfer deliberately omits Content-Length. The limit must
        // count received bytes, including chunks after a valid JSON document.
        let mut wire =
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n".to_vec();
        for chunk in body.chunks(4096) {
            wire.extend_from_slice(format!("{:x}\r\n", chunk.len()).as_bytes());
            wire.extend_from_slice(chunk);
            wire.extend_from_slice(b"\r\n");
        }
        wire.extend_from_slice(b"0\r\n\r\n");
        let mut server = MockServer::start(wire, false).await;
        let result = fetch_markets_request(client.get(&server.url)).await;
        server.finish().await;
        if length == MAX_RESPONSE_BYTES {
            let data = result.expect("catalog at byte limit");
            assert_eq!(data.indexed_through_block, 1234);
            assert!(data.markets.is_empty());
        } else {
            assert_eq!(
                result.expect_err("overflow"),
                "Papertrade market response is too large."
            );
        }
    }
}

#[tokio::test]
async fn papertrade_http_timeouts_distinguish_headers_from_stalled_body() {
    for (wire, expected) in [
        (Vec::new(), "Could not reach Papertrade. Try again."),
        (
            response(200, b"{", 100),
            "Could not read Papertrade markets. Try again.",
        ),
    ] {
        let mut server = MockServer::start(wire, true).await;
        let request = client(Duration::from_millis(250)).get(&server.url);
        let fetch = tokio::spawn(fetch_markets_request(request));
        tokio::time::timeout(Duration::from_secs(2), &mut server.ready)
            .await
            .expect("server received request")
            .expect("server ready");
        // The server remains open until after the client has timed out; no
        // scheduling-dependent sleeps or connection resets cause the error.
        let error = tokio::time::timeout(Duration::from_secs(2), fetch)
            .await
            .expect("client timeout deadline")
            .expect("fetch task")
            .expect_err("request timed out");
        server.finish().await;
        assert_eq!(error, expected);
        assert!(!error.contains(&server.url));
    }
}

#[tokio::test]
async fn papertrade_http_rejects_truncated_and_malformed_bodies() {
    let client = client(Duration::from_secs(2));
    for (body, length, expected) in [
        (
            EMPTY_CATALOG,
            EMPTY_CATALOG.len() + 1,
            "Could not read Papertrade markets. Try again.",
        ),
        (
            "private-sentinel",
            16,
            "Papertrade returned an invalid market catalog.",
        ),
    ] {
        let mut server = MockServer::start(response(200, body.as_bytes(), length), false).await;
        let error = fetch_markets_request(client.get(&server.url))
            .await
            .expect_err("invalid body");
        server.finish().await;
        assert_eq!(error, expected);
    }
}

#[tokio::test]
#[ignore = "reads the public Papertrade market endpoint"]
async fn papertrade_public_catalog_live() {
    let markets = fetch_markets().await.expect("public market catalog");
    assert!(markets.indexed_through_block > 0);
    assert!(!markets.markets.is_empty());
}

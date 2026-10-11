use super::*;
use crate::builder_codes::BuilderFee;
use tokio::net::TcpListener;

fn session(host: &str) -> ApprovalSession {
    ApprovalSession::new(
        host.into(),
        "a".repeat(64),
        "0x1111111111111111111111111111111111111111",
        &BuilderFee::new("0x2222222222222222222222222222222222222222", 10).expect("valid builder"),
        1_700_000_000_000,
    )
}

fn callback(body: &str, origin: &str) -> String {
    format!(
        "POST $PATH/complete HTTP/1.1\r\nHost: $HOST\r\nOrigin: {origin}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

async fn exchange(template: &str) -> (Option<Completion>, String) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("local listener");
    let address = listener.local_addr().expect("listener address");
    let session = session(&address.to_string());
    let request = template
        .replace("$HOST", &session.host)
        .replace("$ORIGIN", &session.origin)
        .replace("$PATH", &session.path);
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("local connection");
        handle_connection(&mut stream, &session).await
    });
    let mut client = TcpStream::connect(address)
        .await
        .expect("connect local listener");
    client
        .write_all(request.as_bytes())
        .await
        .expect("send request");
    client.shutdown().await.expect("finish request");
    let mut response = String::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        client.read_to_string(&mut response),
    )
    .await
    .expect("bounded response")
    .expect("read response");
    (server.await.expect("server completed"), response)
}

#[test]
fn malformed_or_ambiguous_http_headers_are_rejected() {
    for head in [
        "GET / HTTP/1.0\r\nHost: localhost\r\n\r\n",
        "GET / HTTP/1.1\r\nHost: localhost\r\nhOSt: elsewhere\r\n\r\n",
        "GET / HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n",
        "GET / HTTP/1.1\r\nHost: localhost\r\n bad: folded\r\n\r\n",
        "GET / HTTP/1.1\r\nHost: localhost\nOrigin: bad\r\n\r\n",
        "GET / HTTP/1.1\r\n\r\n",
        "GET  / HTTP/1.1\r\nHost: localhost\r\n\r\n",
    ] {
        assert!(parse_head(head.as_bytes()).is_err());
    }
}

#[tokio::test]
async fn only_private_session_path_returns_page_with_security_headers() {
    let (completion, response) = exchange("GET $PATH HTTP/1.1\r\nHost: $HOST\r\n\r\n").await;
    assert_eq!(completion, None);
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("Cache-Control: no-store\r\n"));
    assert!(response.contains("Referrer-Policy: no-referrer\r\n"));
    assert!(response.contains("frame-ancestors 'none'"));
    assert!(!response.contains("Access-Control-Allow-Origin"));
    assert!(response.contains("Connect wallet and approve"));

    let (_, response) = exchange("GET / HTTP/1.1\r\nHost: $HOST\r\n\r\n").await;
    assert!(response.starts_with("HTTP/1.1 404 Not Found\r\n"));
    assert!(!response.contains("0x1111111111111111111111111111111111111111"));
    assert!(
        !response.contains(&"a".repeat(64)),
        "error headers must not expose the session token"
    );
}

#[tokio::test]
async fn host_rebinding_and_cross_origin_requests_cannot_read_or_complete_session() {
    for request in [
        "GET $PATH HTTP/1.1\r\nHost: attacker.example\r\n\r\n".to_string(),
        "GET $PATH HTTP/1.1\r\nHost: $HOST\r\nOrigin: https://attacker.example\r\n\r\n".to_string(),
        "GET $PATH HTTP/1.1\r\nHost: $HOST\r\nSec-Fetch-Site: cross-site\r\n\r\n".to_string(),
        callback("{\"status\":\"approved\"}", "http://localhost:12345"),
        callback("{\"status\":\"approved\"}", "null"),
        callback("{\"status\":\"approved\"}", "https://attacker.example"),
        callback("{\"status\":\"approved\"}", "$ORIGIN").replace("Origin: $ORIGIN\r\n", ""),
    ] {
        let (completion, response) = exchange(&request).await;
        assert_eq!(completion, None);
        assert!(response.starts_with("HTTP/1.1 403 Forbidden\r\n"));
        assert!(!response.contains("0x1111111111111111111111111111111111111111"));
        assert!(
            !response.contains(&"a".repeat(64)),
            "error headers must not expose the session token"
        );
    }
}

#[tokio::test]
async fn completion_requires_exact_same_origin_json_contract() {
    for (status, expected) in [
        ("approved", Completion::Approved),
        ("submitted", Completion::Submitted),
        ("cancelled", Completion::Cancelled),
    ] {
        let (completion, response) = exchange(&callback(
            &format!("{{\"status\":\"{status}\"}}"),
            "$ORIGIN",
        ))
        .await;
        assert_eq!(completion, Some(expected));
        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    }
    for body in [
        "{\"status\":\"approved\",\"action\":{}}",
        "{\"status\":\"other\"}",
        "null",
        "not JSON",
    ] {
        let (completion, response) = exchange(&callback(body, "$ORIGIN")).await;
        assert_eq!(completion, None);
        assert!(response.starts_with("HTTP/1.1 400 Bad Request\r\n"));
    }
    let (completion, response) = exchange(
        &callback("{\"status\":\"approved\"}", "$ORIGIN")
            .replace("Content-Type: application/json", "Content-Type: text/plain"),
    )
    .await;
    assert_eq!(completion, None);
    assert!(response.starts_with("HTTP/1.1 403 Forbidden\r\n"));
}

#[tokio::test]
async fn oversized_truncated_and_pipelined_requests_are_rejected() {
    for request in [
        "POST $PATH/complete HTTP/1.1\r\nHost: $HOST\r\nContent-Length: 16384\r\n\r\n".to_string(),
        "POST $PATH/complete HTTP/1.1\r\nHost: $HOST\r\nContent-Length: 18446744073709551615\r\n\r\n".to_string(),
        "POST $PATH/complete HTTP/1.1\r\nHost: $HOST\r\nContent-Length: +1\r\n\r\nx".to_string(),
        "POST $PATH/complete HTTP/1.1\r\nHost: $HOST\r\nContent-Length: 20\r\n\r\nshort".to_string(),
        "GET $PATH HTTP/1.1\r\nHost: $HOST\r\n\r\npipelined".to_string(),
        format!("GET / HTTP/1.1\r\nHost: $HOST\r\nX-Large: {}\r\n\r\n", "a".repeat(MAX_REQUEST_BYTES)),
    ] {
        let (completion, response) = exchange(&request).await;
        assert_eq!(completion, None);
        assert!(response.starts_with("HTTP/1.1 400 Bad Request\r\n"));
    }
}

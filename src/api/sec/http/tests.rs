use super::*;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn mock_response(
    status: &str,
    body: &str,
    content_length: usize,
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind SEC test server");
    let url = format!(
        "http://{}/fixture",
        listener.local_addr().expect("SEC test server address")
    );
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {content_length}\r\nContent-Type: text/plain; charset=utf-8\r\nConnection: close\r\n\r\n{body}"
    );
    let task = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(5), async move {
            let (mut stream, _) = listener.accept().await.expect("accept SEC test request");
            let mut request = Vec::new();
            loop {
                let mut bytes = [0; 1024];
                let count = stream
                    .read(&mut bytes)
                    .await
                    .expect("read SEC test request");
                assert!(count > 0, "request ended before its headers");
                request.extend_from_slice(&bytes[..count]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
                assert!(
                    request.len() < 16_384,
                    "request headers exceeded test limit"
                );
            }
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write SEC test response");
            String::from_utf8(request).expect("UTF-8 SEC test request")
        })
        .await
        .expect("SEC test server timeout")
    });
    (url, task)
}

async fn assert_sec_request(task: tokio::task::JoinHandle<String>) {
    let request = task.await.expect("SEC test server task");
    assert!(request.starts_with("GET /fixture HTTP/1.1\r\n"));
    // Do not print the configured user agent or captured request on failure.
    assert!(request.contains(&format!("\r\nuser-agent: {}\r\n", sec_user_agent())));
}

#[tokio::test]
async fn sec_readers_preserve_get_headers_and_body_decoding() {
    let body = r#"{"value":42}"#;
    let (url, task) = mock_response("200 OK", body, body.len()).await;
    let json = sec_get_json::<serde_json::Value>(&url)
        .await
        .expect("SEC JSON response");
    assert_eq!(json, serde_json::json!({"value": 42}));
    assert_sec_request(task).await;

    let body = "Revenue — quarter results";
    let (url, task) = mock_response("200 OK", body, body.len()).await;
    assert_eq!(sec_get_text(&url).await.expect("SEC text response"), body);
    assert_sec_request(task).await;
}

#[tokio::test]
async fn sec_readers_check_status_before_decoding_and_keep_distinct_errors() {
    for status in [
        "403 Forbidden",
        "429 Too Many Requests",
        "503 Service Unavailable",
    ] {
        // A short, malformed body would fail decoding if status were not checked first.
        let (url, task) = mock_response(status, "incomplete", 100).await;
        assert_eq!(
            sec_get_json::<serde_json::Value>(&url)
                .await
                .expect_err("HTTP failure"),
            format!("SEC request to EDGAR API returned {status}")
        );
        assert_sec_request(task).await;
        let (url, task) = mock_response(status, "incomplete", 100).await;
        assert_eq!(
            sec_get_text(&url).await.expect_err("HTTP failure"),
            format!("SEC request to EDGAR API returned {status}")
        );
        assert_sec_request(task).await;
    }

    let body = "invalid JSON";
    let (url, task) = mock_response("200 OK", body, body.len()).await;
    assert!(
        sec_get_json::<serde_json::Value>(&url)
            .await
            .expect_err("malformed JSON")
            .starts_with("SEC response parse failed: ")
    );
    assert_sec_request(task).await;
    let (url, task) = mock_response("200 OK", "incomplete", 100).await;
    assert!(
        sec_get_text(&url)
            .await
            .expect_err("incomplete text body")
            .starts_with("SEC text response parse failed: ")
    );
    assert_sec_request(task).await;

    assert!(
        sec_get_json::<serde_json::Value>("://invalid")
            .await
            .expect_err("invalid URL")
            .starts_with("SEC request failed: ")
    );
    assert!(
        sec_get_text("://invalid")
            .await
            .expect_err("invalid URL")
            .starts_with("SEC request failed: ")
    );
}

use super::{fetch_portfolio_history_from_url, parse_history_points, parse_portfolio_bucket};
use crate::account_analytics::PortfolioHistory;

async fn one_shot_info_server(status_line: &str, body: &str) -> String {
    one_shot_info_result(status_line, body)
        .await
        .expect_err("non-success response should fail")
}

async fn one_shot_info_result(status_line: &str, body: &str) -> Result<PortfolioHistory, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let status_line = status_line.to_string();
    let body = body.to_string();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut buf = [0_u8; 4096];
        let _ = socket.read(&mut buf).await.expect("read request");
        let response = format!(
            "HTTP/1.1 {status_line}\r\n\
            content-type: text/plain\r\n\
            content-length: {}\r\n\
            connection: close\r\n\r\n\
            {body}",
            body.len()
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write response");
    });

    let result = fetch_portfolio_history_from_url(
        reqwest::Client::new(),
        &format!("http://{addr}/info"),
        "0xabc".to_string(),
    )
    .await;
    server.await.expect("server task");
    result
}

#[tokio::test]
async fn portfolio_fetch_reports_http_status_before_json_parse() {
    let err = one_shot_info_server("429 Too Many Requests", "rate limited").await;

    assert!(err.contains("portfolio request failed with HTTP 429 Too Many Requests"));
    assert!(err.contains("rate limited"));
    assert!(!err.contains("parse failed"));
}

#[tokio::test]
async fn portfolio_non_array_unicode_preview_does_not_panic() {
    let body = serde_json::to_string(&"€".repeat(100)).expect("fixture should serialize");
    let err = one_shot_info_result("200 OK", &body)
        .await
        .expect_err("non-array portfolio response should fail");

    assert!(err.contains("portfolio response was not an array"));
}

#[tokio::test]
async fn portfolio_error_response_redacts_sensitive_values() {
    let body = serde_json::json!({
        "error": "upstream echoed Authorization: Token token-secret refreshToken=\"refresh-secret\" user=0xabc0000000000000000000000000000000000000"
    })
    .to_string();
    let err = one_shot_info_result("200 OK", &body)
        .await
        .expect_err("portfolio error response should fail");

    assert!(err.contains("portfolio error:"));
    assert!(err.contains("Authorization: Token <redacted>"));
    assert!(err.contains("<redacted-hex>"));
    for secret in [
        "token-secret",
        "refresh-secret",
        "abc0000000000000000000000000000000000000",
    ] {
        assert!(!err.contains(secret), "portfolio error leaked {secret}");
    }
}

#[test]
fn history_points_skip_malformed_numeric_values() {
    let raw = serde_json::json!([
        [1_000, "12.5"],
        [2_000, "bad"],
        [3_000, "NaN"],
        [4_000, 14.25],
        [5_000, "1,234"],
        [0, "99"],
        ["bad-ts", "100"]
    ]);

    assert_eq!(
        parse_history_points(Some(&raw)),
        vec![(1_000, 12.5), (4_000, 14.25)]
    );
}

#[test]
fn portfolio_bucket_tracks_skipped_points_and_invalid_volume() {
    let raw = serde_json::json!({
        "accountValueHistory": [
            [1_000, "100"],
            [2_000, "bad"]
        ],
        "pnlHistory": [
            [1_000, "1"],
            [2_000, "NaN"]
        ],
        "vlm": "bad"
    });
    let bucket = parse_portfolio_bucket(raw.as_object().expect("bucket object"));

    assert_eq!(bucket.account_value_history, vec![(1_000, 100.0)]);
    assert_eq!(bucket.pnl_history, vec![(1_000, 1.0)]);
    assert_eq!(bucket.skipped_invalid_points, 2);
    assert_eq!(bucket.vlm, None);
    assert!(bucket.invalid_vlm);
}

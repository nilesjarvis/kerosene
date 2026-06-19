use super::{account_analytics_preview, post_info_json};

async fn one_shot_response(status_line: &str, content_type: &str, body: &str) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = match TcpListener::bind("127.0.0.1:0").await {
        Ok(listener) => listener,
        Err(error) => panic!("bind HTTP fixture server: {error}"),
    };
    let addr = match listener.local_addr() {
        Ok(addr) => addr,
        Err(error) => panic!("read HTTP fixture server address: {error}"),
    };
    let status_line = status_line.to_string();
    let content_type = content_type.to_string();
    let body = body.to_string();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener
            .accept()
            .await
            .map_err(|error| format!("accept HTTP fixture connection: {error}"))?;
        let mut buf = [0_u8; 4096];
        socket
            .read(&mut buf)
            .await
            .map_err(|error| format!("read HTTP fixture request: {error}"))?;
        let response = format!(
            "HTTP/1.1 {status_line}\r\n\
            content-type: {content_type}\r\n\
            content-length: {}\r\n\
            connection: close\r\n\r\n\
            {body}",
            body.len()
        );
        socket
            .write_all(response.as_bytes())
            .await
            .map_err(|error| format!("write HTTP fixture response: {error}"))?;
        Ok::<(), String>(())
    });

    let client = reqwest::Client::new();
    let result: Result<serde_json::Value, String> = post_info_json(
        &client,
        &format!("http://{addr}/info"),
        "portfolio",
        serde_json::json!({"type": "portfolio", "user": "0xabc"}),
    )
    .await;
    match server.await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => panic!("{error}"),
        Err(error) => panic!("HTTP fixture server task failed: {error}"),
    }
    match result {
        Ok(value) => panic!("response should fail: {value:?}"),
        Err(error) => error,
    }
}

#[tokio::test]
async fn post_info_json_keeps_parse_error_for_successful_invalid_json() {
    let err = one_shot_response("200 OK", "application/json", "not json").await;

    assert!(err.contains("portfolio parse failed"));
    assert!(!err.contains("request failed with HTTP"));
}

#[tokio::test]
async fn post_info_json_reports_http_status_and_body_preview_before_json_parse() {
    let err = one_shot_response(
        "429 Too Many Requests",
        "text/plain",
        "rate limited, retry later",
    )
    .await;

    assert!(err.contains("portfolio request failed with HTTP 429 Too Many Requests"));
    assert!(err.contains("rate limited, retry later"));
    assert!(!err.contains("parse failed"));
}

#[test]
fn account_analytics_preview_redacts_sensitive_response_values() {
    let preview = account_analytics_preview(
        "upstream echoed Authorization: Basic basic-secret accessToken=\"access-secret\" user=0xabc0000000000000000000000000000000000000",
    );

    assert!(preview.contains("Authorization: Basic <redacted>"));
    assert!(preview.contains("<redacted-hex>"));
    for secret in [
        "basic-secret",
        "access-secret",
        "abc0000000000000000000000000000000000000",
    ] {
        assert!(
            !preview.contains(secret),
            "account analytics preview leaked {secret}"
        );
    }
}

#[tokio::test]
async fn post_info_json_reports_empty_http_error_without_parse_failure() {
    let err = one_shot_response("500 Internal Server Error", "text/plain", "").await;

    assert_eq!(
        err,
        "portfolio request failed with HTTP 500 Internal Server Error"
    );
}

use serde::de::DeserializeOwned;
use serde_json::Value;

const HTTP_ERROR_PREVIEW_CHARS: usize = 180;

// ---------------------------------------------------------------------------
// Account Analytics HTTP Helpers
// ---------------------------------------------------------------------------

pub(super) async fn post_info_json<T>(
    client: &reqwest::Client,
    url: &str,
    label: &'static str,
    payload: Value,
) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let response = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("{label} request failed: {e}"))?;

    response_json(label, response).await
}

pub(super) async fn response_json<T>(
    label: &'static str,
    response: reqwest::Response,
) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let preview = body
            .chars()
            .take(HTTP_ERROR_PREVIEW_CHARS)
            .collect::<String>();
        return if preview.is_empty() {
            Err(format!("{label} request failed with HTTP {status}"))
        } else {
            Err(format!(
                "{label} request failed with HTTP {status}: {preview}"
            ))
        };
    }

    response
        .json::<T>()
        .await
        .map_err(|e| format!("{label} parse failed: {e}"))
}

pub(super) async fn optional_response_value(
    response: Result<reqwest::Response, reqwest::Error>,
) -> Option<Value> {
    let response = response.ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.json::<Value>().await.ok()
}

#[cfg(test)]
mod tests {
    use super::post_info_json;

    async fn one_shot_error_response(status_line: &str, body: &str) -> String {
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
                "HTTP/1.1 {status_line}\r\ncontent-type: text/plain\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });

        let client = reqwest::Client::new();
        let result: Result<serde_json::Value, String> = post_info_json(
            &client,
            &format!("http://{addr}/info"),
            "portfolio",
            serde_json::json!({"type": "portfolio", "user": "0xabc"}),
        )
        .await;
        server.await.expect("server task");
        result.expect_err("non-success response should fail")
    }

    async fn one_shot_parse_response(status_line: &str, body: &str) -> String {
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
                "HTTP/1.1 {status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });

        let client = reqwest::Client::new();
        let result: Result<serde_json::Value, String> = post_info_json(
            &client,
            &format!("http://{addr}/info"),
            "portfolio",
            serde_json::json!({"type": "portfolio", "user": "0xabc"}),
        )
        .await;
        server.await.expect("server task");
        result.expect_err("invalid successful JSON response should fail")
    }

    #[tokio::test]
    async fn post_info_json_keeps_parse_error_for_successful_invalid_json() {
        let err = one_shot_parse_response("200 OK", "not json").await;

        assert!(err.contains("portfolio parse failed"));
        assert!(!err.contains("request failed with HTTP"));
    }

    #[tokio::test]
    async fn post_info_json_reports_http_status_and_body_preview_before_json_parse() {
        let err =
            one_shot_error_response("429 Too Many Requests", "rate limited, retry later").await;

        assert!(err.contains("portfolio request failed with HTTP 429 Too Many Requests"));
        assert!(err.contains("rate limited, retry later"));
        assert!(!err.contains("parse failed"));
    }

    #[tokio::test]
    async fn post_info_json_reports_empty_http_error_without_parse_failure() {
        let err = one_shot_error_response("500 Internal Server Error", "").await;

        assert_eq!(
            err,
            "portfolio request failed with HTTP 500 Internal Server Error"
        );
    }
}

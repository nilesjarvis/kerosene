use super::*;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn text_transport_preserves_requests_and_failure_precedence() {
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("fixture client");
    for (context, scope) in [
        ("HyperDash", "liquidation levels"),
        ("HyperDash heatmap", "heatmap"),
        ("HyperDash positioning", "positioning"),
    ] {
        for (status, body, truncated) in [
            (200, "  Σ → 🦀\n ", false),
            (206, "partial result", false),
            (204, "", false),
            (401, "private provider detail", false),
            (403, "private provider detail", false),
            (429, "api_key=fixture-secret", false),
            (502, "api_key=fixture-secret", false),
            (200, "partial", true),
            (401, "partial", true),
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("fixture listener");
            let address = listener.local_addr().expect("fixture address");
            let server = tokio::spawn(async move {
                tokio::time::timeout(Duration::from_secs(3), async move {
                    let (mut stream, _) = listener.accept().await.expect("fixture connection");
                    let mut request = Vec::new();
                    while !request.windows(7).any(|part| part == b"PAYLOAD") {
                        let mut buffer = [0; 1_024];
                        let read = stream.read(&mut buffer).await.expect("fixture request");
                        assert!(read > 0 && request.len() < 8_192);
                        request.extend_from_slice(&buffer[..read]);
                    }
                    assert!(request.starts_with(b"POST /fixture HTTP/1.1\r\n"));
                    let request = String::from_utf8(request).expect("ASCII request");
                    assert!(request.contains("x-fixture: preserved\r\n"));
                    let length = body.len() + usize::from(truncated);
                    let response = format!(
                        "HTTP/1.1 {status} Fixture\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n{body}"
                    );
                    stream.write_all(response.as_bytes()).await.expect("fixture response");
                })
                .await
                .expect("fixture deadline");
            });

            let result = request_text(
                client
                    .post(format!("http://{address}/fixture"))
                    .header("x-fixture", "preserved")
                    .body("PAYLOAD"),
                context,
                scope,
            )
            .await;
            server.await.expect("fixture task");

            if truncated {
                assert!(
                    result
                        .expect_err("truncated response")
                        .starts_with(&format!("Failed to read {context} response:"))
                );
            } else if (200..300).contains(&status) {
                assert_eq!(result.expect("successful response"), body);
            } else {
                let status = reqwest::StatusCode::from_u16(status).expect("fixture status");
                let expected = if status == reqwest::StatusCode::UNAUTHORIZED
                    || status == reqwest::StatusCode::FORBIDDEN
                {
                    format!(
                        "HyperDash {scope} authentication failed (HTTP {status}). Check the API key in Settings > Integrations."
                    )
                } else {
                    format!("HyperDash {scope} request failed (HTTP {status}): api_key=<redacted>")
                };
                assert_eq!(result.expect_err("HTTP failure"), expected);
            }
        }
        let error = request_text(client.get("not a URL"), context, scope)
            .await
            .expect_err("invalid request builder");
        assert!(error.starts_with(&format!("{context} request failed:")));
    }
}

use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1A\n";

async fn mock_response(
    status: &str,
    headers: &str,
    body: Vec<u8>,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind Telegram image test server");
    let url = format!(
        "http://{}/fixture",
        listener.local_addr().expect("test server address")
    );
    let mut response =
        format!("HTTP/1.1 {status}\r\n{headers}Connection: close\r\n\r\n").into_bytes();
    response.extend(body);
    let task = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(5), async move {
            let (mut stream, _) = listener.accept().await.expect("accept test request");
            let mut request = Vec::new();
            loop {
                let mut bytes = [0; 1024];
                let count = stream.read(&mut bytes).await.expect("read test request");
                assert!(count > 0, "request ended before its headers");
                request.extend_from_slice(&bytes[..count]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
                assert!(request.len() < 16_384, "test request exceeded header limit");
            }
            let request = String::from_utf8(request).expect("UTF-8 test request");
            assert!(request.starts_with("GET /fixture HTTP/1.1\r\n"));
            assert!(request.contains(&format!("\r\nuser-agent: {TELEGRAM_USER_AGENT}\r\n")));
            if let Err(error) = stream.write_all(&response).await {
                // Status/size rejection may close the connection before the body is sent.
                assert!(matches!(
                    error.kind(),
                    std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset
                ));
            }
        })
        .await
        .expect("Telegram image test server timeout");
    });
    (url, task)
}

async fn fetch_fixture(media: bool, url: String) -> Result<Vec<u8>, String> {
    if media {
        fetch_telegram_media_bytes(" @MarketFeed ".to_string(), 37, url).await
    } else {
        fetch_telegram_avatar_bytes(" @MarketFeed ".to_string(), url).await
    }
}

fn image_label(media: bool) -> &'static str {
    if media {
        "@marketfeed/37 media"
    } else {
        "@marketfeed avatar"
    }
}

fn image_limit(media: bool) -> usize {
    if media {
        TELEGRAM_MEDIA_MAX_BODY_BYTES
    } else {
        TELEGRAM_AVATAR_MAX_BODY_BYTES
    }
}

#[tokio::test]
async fn image_fetchers_check_signatures_and_keep_content_type_error_context() {
    for media in [false, true] {
        let (url, task) = mock_response(
            "200 OK",
            "Content-Length: 8\r\nContent-Type: text/plain\r\n",
            PNG_SIGNATURE.to_vec(),
        )
        .await;
        assert_eq!(
            fetch_fixture(media, url).await.expect("image bytes"),
            PNG_SIGNATURE
        );
        task.await.expect("test server task");

        for (content_type, expected) in [
            ("Content-Type: image/png\r\n", "image/png"),
            ("", "unknown content type"),
        ] {
            let (url, task) = mock_response(
                "200 OK",
                &format!("Content-Length: 3\r\n{content_type}"),
                b"bad".to_vec(),
            )
            .await;
            assert_eq!(
                fetch_fixture(media, url).await.expect_err("invalid image"),
                format!(
                    "{} response was not a supported image: {expected}",
                    image_label(media)
                )
            );
            task.await.expect("test server task");
        }
    }
}

#[tokio::test]
async fn image_fetchers_keep_validation_status_size_and_read_error_precedence() {
    for media in [false, true] {
        let label = image_label(media);
        let limit = image_limit(media);
        let headers = format!("Content-Length: {}\r\n", limit + 1);
        for (status, expected) in [
            (
                "404 Not Found",
                format!("{label} request failed with HTTP 404 Not Found"),
            ),
            (
                "200 OK",
                format!("{label} response was too large: more than {limit} bytes"),
            ),
        ] {
            let (url, task) = mock_response(status, &headers, Vec::new()).await;
            assert_eq!(
                fetch_fixture(media, url)
                    .await
                    .expect_err("rejected response"),
                expected
            );
            task.await.expect("test server task");
        }

        let (url, task) = mock_response("200 OK", "Content-Length: 100\r\n", b"bad".to_vec()).await;
        assert!(
            fetch_fixture(media, url)
                .await
                .expect_err("incomplete body")
                .starts_with(&format!("{label} response read failed:"))
        );
        task.await.expect("test server task");
        assert!(
            fetch_fixture(media, "://invalid".to_string())
                .await
                .expect_err("invalid URL")
                .starts_with(&format!("{label} request failed:"))
        );
    }

    assert_eq!(
        fetch_telegram_avatar_bytes("".to_string(), "://invalid".to_string()).await,
        Err("Enter a public Telegram channel".to_string())
    );
    assert_eq!(
        fetch_telegram_media_bytes("".to_string(), 37, "://invalid".to_string()).await,
        Err("Enter a public Telegram channel".to_string())
    );
}

#[tokio::test]
async fn image_fetchers_accept_limit_and_reject_chunked_overflow() {
    for media in [false, true] {
        let limit = image_limit(media);
        for (chunked, len) in [(false, limit), (true, limit), (true, limit + 1)] {
            let mut body = PNG_SIGNATURE.to_vec();
            body.resize(len, 0);
            let headers = if chunked {
                let mut framed = format!("{len:X}\r\n").into_bytes();
                framed.extend(body);
                framed.extend_from_slice(b"\r\n0\r\n\r\n");
                body = framed;
                "Transfer-Encoding: chunked\r\n".to_string()
            } else {
                format!("Content-Length: {len}\r\n")
            };
            let (url, task) = mock_response("200 OK", &headers, body).await;
            let result = fetch_fixture(media, url).await;
            if len <= limit {
                assert_eq!(result.expect("image at size limit").len(), len);
            } else {
                assert_eq!(
                    result.expect_err("streamed overflow"),
                    format!(
                        "{} response was too large: more than {limit} bytes",
                        image_label(media)
                    )
                );
            }
            task.await.expect("test server task");
        }
    }
}

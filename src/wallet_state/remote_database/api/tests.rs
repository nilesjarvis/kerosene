use super::*;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const ADDRESS_A: &str = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ADDRESS_B: &str = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

async fn server(responses: Vec<(u16, String)>) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test server");
    let url = format!("http://{}", listener.local_addr().expect("local address"));
    let handle = tokio::spawn(async move {
        for (index, (status, body)) in responses.into_iter().enumerate() {
            let (mut socket, _) = listener.accept().await.expect("accept request");
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                request.push(socket.read_u8().await.expect("request byte"));
            }
            let request = String::from_utf8(request).expect("HTTP request");
            assert!(request.starts_with("GET /prefix/api/collections/wallets/records?"));
            assert!(request.contains(&format!("page={}", index + 1)));
            assert!(request.contains("perPage=200"));
            assert!(request.contains("expand=entity"));
            assert!(request.contains("sort=id"));
            assert!(request.contains(
                "fields=id%2Caddress%2Clabel%2Ctags%2Cexpand.entity.name%2Cexpand.entity.tags"
            ));
            assert!(!request.to_lowercase().contains("authorization:"));
            socket.write_all(format!(
                "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()
            ).as_bytes()).await.expect("response");
        }
    });
    (format!("{url}/prefix"), handle)
}

fn page(number: usize, total: usize, items: Vec<Value>) -> String {
    json!({"page":number,"perPage":1,"totalPages":total,"totalItems":total,"items":items})
        .to_string()
}

#[test]
fn base_url_normalization_rejects_credentials_and_wrong_endpoints() {
    assert_eq!(
        normalize_base_url(" HTTP://LOCALHOST:8090/base/ ").expect("valid URL"),
        "http://localhost:8090/base"
    );
    assert_eq!(normalize_base_url("  ").expect("disable"), "");
    for invalid in [
        "localhost:8090",
        "file:///tmp/db",
        "https://user:secret@host",
        "https://host?token=secret",
        "https://host/#secret",
        "https://host/api/collections/wallets/records",
        "https://host/_/",
    ] {
        let error = normalize_base_url(invalid).expect_err("invalid base URL");
        assert!(!error.contains("secret"));
    }
}

#[tokio::test]
async fn reads_every_page_and_maps_wallet_and_entity_labels() {
    let (url, handle) = server(vec![
        (200, page(1, 3, vec![json!({"id":"a","address":ADDRESS_A.to_uppercase(),"label":" Vault ","tags":["desk","desk"," "],"expand":{"entity":{"name":"Owner","tags":["desk","entity-tag"]}}})])),
        (200, page(2, 3, vec![json!({"id":"b","address":ADDRESS_B,"label":" ","tags":null,"expand":{"entity":{"name":" Owner "}}})])),
        (200, page(3, 3, vec![json!({"id":"c","address":"non-evm-wallet","chain":"solana","label":"Skipped"})])),
    ]).await;
    let result = fetch_wallets(url).await.expect("complete snapshot");
    handle.await.expect("server");
    assert_eq!(result.entries.len(), 2);
    assert_eq!(result.entries[ADDRESS_A].label, "Vault");
    assert_eq!(result.entries[ADDRESS_A].tags, ["desk", "entity-tag"]);
    assert_eq!(result.entries[ADDRESS_B].label, "Owner");
    assert_eq!(result.skipped, 1);
}

#[tokio::test]
async fn empty_database_is_a_successful_snapshot() {
    let (url, handle) = server(vec![(200, page(1, 0, vec![]))]).await;
    assert!(
        fetch_wallets(url)
            .await
            .expect("empty snapshot")
            .entries
            .is_empty()
    );
    handle.await.expect("server");
}

#[tokio::test]
async fn failed_later_page_never_returns_a_partial_snapshot_or_response_body() {
    for (status, body) in [
        (403, "private-error-sentinel".to_string()),
        (200, "private-error-sentinel".to_string()),
        (200, page(2, 3, vec![json!({"id":"b","address":ADDRESS_B})])),
        (200, page(2, 2, vec![])),
        (200, page(2, 2, vec![json!({"id":"a","address":ADDRESS_B})])),
        (
            200,
            page(
                2,
                2,
                vec![json!({"id":"b","address":ADDRESS_A.to_uppercase()})],
            ),
        ),
        (
            200,
            page(2, 2, vec![json!({"id":"b","label":"Missing address"})]),
        ),
    ] {
        let (url, handle) = server(vec![
            (200, page(1, 2, vec![json!({"id":"a","address":ADDRESS_A})])),
            (status, body),
        ])
        .await;
        let error = fetch_wallets(url).await.err().expect("invalid snapshot");
        assert!(!error.contains("private-error-sentinel"));
        assert!(!error.contains(ADDRESS_A));
        handle.await.expect("server");
    }
}

#[tokio::test]
async fn oversized_response_is_rejected() {
    let (url, handle) = server(vec![(200, " ".repeat(MAX_PAGE_BYTES + 1))]).await;
    assert!(
        fetch_wallets(url)
            .await
            .err()
            .expect("oversized")
            .contains("2 MiB")
    );
    // The client may close the socket before the test server finishes writing.
    handle.abort();
}

#[tokio::test]
async fn stalled_response_times_out_without_leaking_url() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let url = format!("http://{}", listener.local_addr().expect("address"));
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_millis(50))
        .build()
        .expect("client");
    let error = fetch_pages(&client, &url).await.err().expect("timeout");
    assert!(!error.contains(&url));
}

#[tokio::test]
#[ignore = "requires KEROSENE_TEST_WALLET_DATABASE_URL pointing to a readable PocketBase"]
async fn live_remote_wallet_database_matches_contract() {
    let url = std::env::var("KEROSENE_TEST_WALLET_DATABASE_URL").expect("test URL configured");
    let result = fetch_wallets(url).await.expect("complete live snapshot");
    assert!(
        !result.entries.is_empty(),
        "live fixture should contain wallets"
    );
}

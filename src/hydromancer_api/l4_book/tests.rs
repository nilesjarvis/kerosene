use super::*;
use serde_json::json;

const WALLET: &str = "0x1111111111111111111111111111111111111111";

fn payload(coin: &str) -> Vec<u8> {
    let market = (
        coin,
        vec![(WALLET, "100.12345678", "2.5", 10u64)],
        vec![(WALLET, "101", "3", 11u64)],
    );
    let decoded = rmp_serde::to_vec(&market).expect("msgpack");
    let blob = zstd::stream::encode_all(decoded.as_slice(), 1).expect("zstd");
    let mut bytes = Vec::new();
    bytes.extend(1u32.to_le_bytes());
    bytes.extend(100u64.to_le_bytes());
    bytes.extend(1234u64.to_le_bytes());
    bytes.extend((blob.len() as u32).to_le_bytes());
    bytes.extend(blob);
    bytes
}

#[test]
fn decodes_documented_binary_snapshot_for_explicit_market_keys() {
    for coin in ["BTC", "xyz:XYZ100", "@107", "#2870"] {
        let snapshot = decode_snapshot(&payload(coin), coin).expect("snapshot");
        assert_eq!(snapshot.height, 100);
        assert_eq!(snapshot.timestamp_ms, 1234);
        assert_eq!(snapshot.orders.len(), 2);
        assert_eq!(snapshot.orders[0].wallet.as_str(), WALLET);
        assert_eq!(snapshot.orders[0].price, 10_012_345_678);
        assert!(snapshot.orders[0].is_bid);
        assert!(!snapshot.orders[1].is_bid);
    }
}

#[test]
fn truncated_mismatched_and_trailing_snapshots_fail_closed() {
    let bytes = payload("BTC");
    for end in [0, 3, 10, 19, 20, 23, bytes.len() - 1] {
        assert!(decode_snapshot(&bytes[..end], "BTC").is_err());
    }
    assert!(decode_snapshot(&bytes, "ETH").is_err());
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(decode_snapshot(&trailing, "BTC").is_err());
    let mut wrong_count = bytes.clone();
    wrong_count[0] = 2;
    assert!(decode_snapshot(&wrong_count, "BTC").is_err());
    let mut corrupt = bytes;
    corrupt[24] = 0;
    assert!(decode_snapshot(&corrupt, "BTC").is_err());
}

#[test]
fn stream_parser_preserves_priority_and_filters_other_markets() {
    let block = decode_block(&json!({"type":"l4BookUpdates", "data": {"height": 101, "timestamp": 1235, "diffs": [
        {"type":"new", "coin":"BTC", "oid":12, "user":WALLET, "side":"B", "px":"100", "sz":"1", "insertBefore":10},
        {"type":"update", "coin":"BTC", "oid":10, "sz":"0.5"},
        {"type":"remove", "coin":"BTC", "oid":11},
        {"type":"remove", "coin":"ETH", "oid":99}
    ]}}), "BTC").expect("block");
    assert_eq!(block.diffs.len(), 3);
    assert!(matches!(&block.diffs[0], Diff::New(order, Some(10)) if order.oid == 12));
    assert!(matches!(block.diffs[1], Diff::Update(10, 50_000_000)));
    assert!(matches!(block.diffs[2], Diff::Remove(11)));
}

#[test]
fn malformed_updates_never_become_empty_successful_blocks_or_leak_raw_values() {
    for diff in [
        json!({"type":"remove", "coin":"BTC"}),
        json!({"type":"unknown", "coin":"BTC", "oid":1}),
        json!({"type":"new", "coin":"BTC", "oid":1, "user":WALLET, "side":"invalid", "px":"1", "sz":"1"}),
        json!({"type":"new", "coin":"BTC", "oid":1, "user":"private-secret", "side":"B", "px":"1", "sz":"1"}),
        json!({"type":"update", "coin":"BTC", "oid":1, "sz":"private-secret"}),
    ] {
        let result = decode_block(
            &json!({"height":101, "timestamp":1235, "diffs":[diff]}),
            "BTC",
        );
        let error = result.err().expect("invalid block");
        assert!(!error.contains("private-secret"));
        assert!(!error.contains(WALLET));
    }
    assert!(decode_block(&json!({"height":101, "timestamp":1235}), "BTC").is_err());
}

#[test]
fn process_snapshot_gate_does_not_burst_after_idle_or_repeated_retries() {
    let now = Instant::now();
    let mut next = None;
    assert!(reserve_snapshot_at(&mut next, now).is_ok());
    for seconds in 0..31 {
        assert!(reserve_snapshot_at(&mut next, now + Duration::from_secs(seconds)).is_err());
    }
    assert!(reserve_snapshot_at(&mut next, now + Duration::from_secs(31)).is_ok());
    assert!(reserve_snapshot_at(&mut next, now + Duration::from_secs(1000)).is_ok());
    assert!(reserve_snapshot_at(&mut next, now + Duration::from_secs(1000)).is_err());
}

async fn server(status: &str, body: Vec<u8>) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("local address");
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut request = [0u8; 4096];
        let read = socket.read(&mut request).await.expect("request");
        let request = String::from_utf8_lossy(&request[..read]);
        assert!(request.contains("authorization: Bearer synthetic-key"));
        socket
            .write_all(response.as_bytes())
            .await
            .expect("headers");
        socket.write_all(&body).await.expect("body");
    });
    format!("http://{address}/info")
}

#[tokio::test]
async fn http_snapshot_path_decodes_binary_and_redacts_permission_errors() {
    let url = server("200 OK", payload("BTC")).await;
    let result =
        fetch_snapshot_from("BTC".into(), Zeroizing::new("synthetic-key".into()), &url).await;
    assert_eq!(result.ok().expect("decoded snapshot").orders.len(), 2);
    let url = server("403 Forbidden", b"private-response-secret".to_vec()).await;
    let error = fetch_snapshot_from("BTC".into(), Zeroizing::new("synthetic-key".into()), &url)
        .await
        .err()
        .expect("denied");
    assert!(error.permanent);
    assert!(error.message.contains("access denied"));
    assert!(!error.message.contains("private-response-secret"));
}

#[test]
fn retry_after_honors_seconds_dates_and_conservative_fallback() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let mut headers = reqwest::header::HeaderMap::new();
    assert_eq!(retry_after(&headers, now), Duration::from_secs(300));
    headers.insert(reqwest::header::RETRY_AFTER, "60".parse().expect("header"));
    assert_eq!(retry_after(&headers, now), Duration::from_secs(60));
    headers.insert(
        reqwest::header::RETRY_AFTER,
        "Tue, 14 Nov 2023 22:15:20 GMT".parse().expect("header"),
    );
    assert_eq!(retry_after(&headers, now), Duration::from_secs(120));
    headers.insert(reqwest::header::RETRY_AFTER, "0".parse().expect("header"));
    assert_eq!(retry_after(&headers, now), SNAPSHOT_SPACING);
}

use super::*;

#[test]
fn chase_reprice_reads_only_the_orders_dex_that_owns_the_symbol() {
    for (coin, expected_dex) in [("BTC", None), ("@12", None), ("xyz:GOLD", Some("xyz"))] {
        let (dex, payload) = orders_payload("test-account", coin);
        assert_eq!(dex, expected_dex.unwrap_or_default());
        assert_eq!(payload["type"], "frontendOpenOrders");
        assert_eq!(payload["user"], "test-account");
        assert_eq!(payload.get("dex").and_then(Value::as_str), expected_dex);
    }
}

#[tokio::test]
async fn chase_reprice_snapshot_rejects_failed_and_malformed_reads() {
    for (status, body) in [
        (429, "[]"),
        (500, "[]"),
        (200, "null"),
        (200, "{}"),
        (200, "[{\"oid\":42}]"),
    ] {
        let response = http::Response::builder()
            .status(status)
            .body(body)
            .expect("response");
        assert!(
            parse_required_response::<OpenOrder>("frontendOpenOrders", response.into())
                .await
                .is_err()
        );
    }
    let response = http::Response::new("[]");
    assert!(
        parse_required_response::<OpenOrder>("frontendOpenOrders", response.into())
            .await
            .expect("complete empty snapshot")
            .is_empty()
    );
}

#[test]
fn chase_reprice_snapshot_debug_shows_only_counts() {
    let snapshot = ChaseRepriceSnapshot {
        open_orders: vec![],
        fills: vec![],
    };
    assert_eq!(
        format!("{snapshot:?}"),
        "ChaseRepriceSnapshot { open_orders_len: 0, fills_len: 0 }"
    );
}

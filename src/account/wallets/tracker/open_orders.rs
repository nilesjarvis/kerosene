use super::super::super::{HIP3_DEXES, OpenOrder};
use crate::api::API_URL;

/// Fetch open-order count for the wallet tracker's slow/manual order lane.
pub async fn fetch_wallet_tracker_open_order_count(address: String) -> Result<usize, String> {
    let client = crate::api::CLIENT.clone();
    let mut order_futs = Vec::new();
    order_futs.push(
        client
            .post(API_URL)
            .json(&serde_json::json!({"type": "openOrders", "user": address}))
            .send(),
    );

    for dex in HIP3_DEXES {
        order_futs.push(
            client
                .post(API_URL)
                .json(&serde_json::json!({
                    "type": "openOrders",
                    "user": address,
                    "dex": dex
                }))
                .send(),
        );
    }

    let mut open_order_count = 0_usize;
    let mut failures = Vec::new();
    for resp in futures::future::join_all(order_futs).await {
        match resp {
            Ok(response) => {
                if !response.status().is_success() {
                    failures.push(format!("HTTP {}", response.status()));
                    continue;
                }
                match response.json::<Vec<OpenOrder>>().await {
                    Ok(orders) => open_order_count += orders.len(),
                    Err(e) => failures.push(format!("parse failed: {e}")),
                }
            }
            Err(e) => failures.push(format!("request failed: {e}")),
        }
    }

    if failures.is_empty() {
        Ok(open_order_count)
    } else {
        Err(format!(
            "openOrders refresh partially failed: {}",
            failures.join("; ")
        ))
    }
}

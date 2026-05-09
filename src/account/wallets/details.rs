mod hip3;

use self::hip3::{append_hip3_open_orders, append_hip3_positions, fetch_hip3_wallet_details};
use super::super::http::{best_effort_response_vec, post_info_json_with_retries};
use super::super::{
    ClearinghouseState, OpenOrder, SpotClearinghouseState, WalletDetailsData,
    WalletOpenOrderDetail, WalletPositionDetail,
};
use crate::api::API_URL;

/// Fetch detailed watch-only wallet state for a detachable details window.
///
/// This is heavier than `fetch_wallet_tracker_snapshot`, so it is intended for
/// opening/manual refresh. Live updates are layered on via websocket state.
pub async fn fetch_wallet_details(address: String) -> Result<WalletDetailsData, String> {
    let client = crate::api::CLIENT.clone();

    let ch_fut = post_info_json_with_retries(
        client.clone(),
        "clearinghouseState",
        serde_json::json!({"type": "clearinghouseState", "user": address}),
    );
    let spot_fut = post_info_json_with_retries(
        client.clone(),
        "spotClearinghouseState",
        serde_json::json!({"type": "spotClearinghouseState", "user": address}),
    );
    let orders_fut = client
        .post(API_URL)
        .json(&serde_json::json!({"type": "frontendOpenOrders", "user": address}))
        .send();

    let (ch_raw, spot_raw, main_orders_resp) =
        futures::future::join3(ch_fut, spot_fut, orders_fut).await;

    let clearinghouse: ClearinghouseState = serde_json::from_value(ch_raw?)
        .map_err(|e| format!("clearinghouseState deserialize failed: {e}"))?;
    let spot: SpotClearinghouseState = serde_json::from_value(spot_raw?)
        .map_err(|e| format!("spotClearinghouseState deserialize failed: {e}"))?;

    let mut warnings = Vec::new();
    let mut positions: Vec<WalletPositionDetail> = clearinghouse
        .asset_positions
        .iter()
        .cloned()
        .map(|asset_position| WalletPositionDetail {
            dex: String::new(),
            asset_position,
        })
        .collect();

    let mut open_orders: Vec<WalletOpenOrderDetail> = best_effort_response_vec::<OpenOrder>(
        "frontendOpenOrders",
        main_orders_resp,
        &mut warnings,
    )
    .await
    .into_iter()
    .map(|order| WalletOpenOrderDetail {
        dex: String::new(),
        order,
    })
    .collect();

    let (hip3_ch_results, hip3_order_results) =
        fetch_hip3_wallet_details(client.clone(), address).await;

    append_hip3_positions(hip3_ch_results, &mut positions, &mut warnings).await;
    append_hip3_open_orders(hip3_order_results, &mut open_orders, &mut warnings).await;

    let fetched_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    Ok(WalletDetailsData {
        clearinghouse,
        spot,
        positions,
        open_orders,
        warnings,
        fetched_at_ms,
    })
}

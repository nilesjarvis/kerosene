use super::super::super::{
    ClearinghouseState, HIP3_DEXES, OpenOrder, WalletOpenOrderDetail, WalletPositionDetail,
};
use crate::api::API_URL;

use serde_json::Value;

type Hip3ResponseResults = Vec<(String, Result<reqwest::Response, reqwest::Error>)>;

pub(super) async fn fetch_hip3_wallet_details(
    client: reqwest::Client,
    address: String,
) -> (Hip3ResponseResults, Hip3ResponseResults) {
    let mut hip3_ch_futs = Vec::new();
    let mut hip3_order_futs = Vec::new();
    for dex in HIP3_DEXES {
        hip3_ch_futs.push((
            (*dex).to_string(),
            client
                .post(API_URL)
                .json(&serde_json::json!({
                    "type": "clearinghouseState",
                    "user": address,
                    "dex": dex
                }))
                .send(),
        ));
        hip3_order_futs.push((
            (*dex).to_string(),
            client
                .post(API_URL)
                .json(&serde_json::json!({
                    "type": "frontendOpenOrders",
                    "user": address,
                    "dex": dex
                }))
                .send(),
        ));
    }

    futures::future::join(
        futures::future::join_all(
            hip3_ch_futs
                .into_iter()
                .map(|(dex, request)| async move { (dex, request.await) }),
        ),
        futures::future::join_all(
            hip3_order_futs
                .into_iter()
                .map(|(dex, request)| async move { (dex, request.await) }),
        ),
    )
    .await
}

pub(super) async fn append_hip3_positions(
    hip3_ch_results: Hip3ResponseResults,
    positions: &mut Vec<WalletPositionDetail>,
    warnings: &mut Vec<String>,
) {
    for (dex, resp) in hip3_ch_results {
        match resp {
            Ok(response) if response.status().is_success() => {
                match response.json::<Value>().await {
                    Ok(raw) => match serde_json::from_value::<ClearinghouseState>(raw) {
                        Ok(ch) => {
                            positions.extend(ch.asset_positions.into_iter().map(
                                |asset_position| WalletPositionDetail {
                                    dex: dex.clone(),
                                    asset_position,
                                },
                            ));
                        }
                        Err(e) => {
                            warnings.push(format!("{dex} clearinghouseState parse failed: {e}"))
                        }
                    },
                    Err(e) => warnings.push(format!(
                        "{dex} clearinghouseState response parse failed: {e}"
                    )),
                }
            }
            Ok(response) => warnings.push(format!(
                "{dex} clearinghouseState failed with HTTP {}",
                response.status()
            )),
            Err(e) => warnings.push(format!("{dex} clearinghouseState request failed: {e}")),
        }
    }
}

pub(super) async fn append_hip3_open_orders(
    hip3_order_results: Hip3ResponseResults,
    open_orders: &mut Vec<WalletOpenOrderDetail>,
    warnings: &mut Vec<String>,
) {
    for (dex, resp) in hip3_order_results {
        match resp {
            Ok(response) if response.status().is_success() => {
                match response.json::<Vec<OpenOrder>>().await {
                    Ok(orders) => {
                        open_orders.extend(orders.into_iter().map(|order| WalletOpenOrderDetail {
                            dex: dex.clone(),
                            order,
                        }));
                    }
                    Err(e) => warnings.push(format!("{dex} frontendOpenOrders parse failed: {e}")),
                }
            }
            Ok(response) => warnings.push(format!(
                "{dex} frontendOpenOrders failed with HTTP {}",
                response.status()
            )),
            Err(e) => warnings.push(format!("{dex} frontendOpenOrders request failed: {e}")),
        }
    }
}

use super::super::super::{
    AccountDataFetchScope, ClearinghouseState, HIP3_DEXES, OpenOrder, WalletOpenOrderDetail,
    WalletPositionDetail, normalize_dex_asset_position_coins, normalize_dex_open_order_coins,
};
use crate::api::API_URL;
use crate::api::proxy::HyperliquidRequestExt;

use serde_json::Value;

type Hip3ResponseResults<'a> = Vec<(&'a str, Result<reqwest::Response, String>)>;

pub(super) async fn fetch_hip3_wallet_details<'a>(
    client: &reqwest::Client,
    address: String,
    scope: &'a AccountDataFetchScope,
) -> (Hip3ResponseResults<'a>, Hip3ResponseResults<'a>) {
    let mut hip3_ch_futs = Vec::new();
    let mut hip3_order_futs = Vec::new();
    for dex in scope.hip3_dexes(HIP3_DEXES) {
        hip3_ch_futs.push((
            dex,
            client
                .post(API_URL)
                .json(&serde_json::json!({
                    "type": "clearinghouseState",
                    "user": address,
                    "dex": dex
                }))
                .send_info(),
        ));
        hip3_order_futs.push((
            dex,
            client
                .post(API_URL)
                .json(&serde_json::json!({
                    "type": "frontendOpenOrders",
                    "user": address,
                    "dex": dex
                }))
                .send_info(),
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
    hip3_ch_results: Hip3ResponseResults<'_>,
    positions: &mut Vec<WalletPositionDetail>,
    warnings: &mut Vec<String>,
) {
    for (dex, resp) in hip3_ch_results {
        match resp {
            Ok(response) if response.status().is_success() => {
                match response.json::<Value>().await {
                    Ok(raw) => match serde_json::from_value::<ClearinghouseState>(raw) {
                        Ok(mut ch) => {
                            normalize_dex_asset_position_coins(dex, &mut ch.asset_positions);
                            positions.extend(ch.asset_positions.into_iter().map(
                                |asset_position| WalletPositionDetail {
                                    dex: dex.to_string(),
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
    hip3_order_results: Hip3ResponseResults<'_>,
    open_orders: &mut Vec<WalletOpenOrderDetail>,
    warnings: &mut Vec<String>,
) {
    for (dex, resp) in hip3_order_results {
        match resp {
            Ok(response) if response.status().is_success() => {
                match response.json::<Vec<OpenOrder>>().await {
                    Ok(mut orders) => {
                        normalize_dex_open_order_coins(dex, &mut orders);
                        open_orders.extend(orders.into_iter().map(|order| WalletOpenOrderDetail {
                            dex: dex.to_string(),
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

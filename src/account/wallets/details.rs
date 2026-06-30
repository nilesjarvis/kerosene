mod hip3;

use self::hip3::{append_hip3_open_orders, append_hip3_positions, fetch_hip3_wallet_details};
use super::super::http::{best_effort_response_vec, post_info_json_with_retries};
use super::super::{
    AccountDataFetchScope, ClearinghouseState, HIP3_DEXES, OpenOrder, SpotClearinghouseState,
    UserFill, WalletDetailsData, WalletOpenOrderDetail, WalletPositionDetail,
    fetch_hydromancer_frontend_open_orders_scoped, fetch_hydromancer_portfolio_state,
    fetch_hydromancer_user_fills,
};
use crate::api::API_URL;
use crate::app_time::now_ms;
use crate::helpers::parse_finite_number;
use zeroize::Zeroizing;

/// Fetch detailed watch-only wallet state for a detachable details window.
///
/// This is heavier than `fetch_wallet_tracker_snapshot`, so it is intended for
/// opening/manual refresh. Live updates are layered on via websocket state.
pub async fn fetch_wallet_details_scoped(
    address: String,
    scope: AccountDataFetchScope,
) -> Result<WalletDetailsData, String> {
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
    let fetch_main_orders = scope.fetches_main_open_orders();
    let orders_fut = async {
        if fetch_main_orders {
            Some(
                client
                    .post(API_URL)
                    .json(&serde_json::json!({"type": "frontendOpenOrders", "user": address}))
                    .send()
                    .await,
            )
        } else {
            None
        }
    };
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

    let mut open_orders: Vec<WalletOpenOrderDetail> = match main_orders_resp {
        Some(main_orders_resp) => best_effort_response_vec::<OpenOrder>(
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
        .collect(),
        None => Vec::new(),
    };
    let fills = fetch_wallet_user_fills_if_needed(&address, &spot, &mut warnings).await;

    let (hip3_ch_results, hip3_order_results) =
        fetch_hip3_wallet_details(client.clone(), address, &scope).await;

    append_hip3_positions(hip3_ch_results, &mut positions, &mut warnings).await;
    append_hip3_open_orders(hip3_order_results, &mut open_orders, &mut warnings).await;

    Ok(WalletDetailsData {
        clearinghouse,
        spot,
        positions,
        open_orders,
        fills,
        warnings,
        fetched_at_ms: now_ms(),
    })
}

pub async fn fetch_wallet_details_scoped_with_provider(
    address: String,
    scope: AccountDataFetchScope,
    provider: crate::config::ReadDataProvider,
    hydromancer_api_key: Zeroizing<String>,
) -> Result<WalletDetailsData, String> {
    if provider != crate::config::ReadDataProvider::Hydromancer {
        return fetch_wallet_details_scoped(address, scope).await;
    }

    let api_key = Zeroizing::new(hydromancer_api_key.trim().to_string());
    if api_key.is_empty() {
        return fetch_wallet_details_scoped(address, scope).await;
    }

    match fetch_wallet_details_scoped_hydromancer(address.clone(), scope.clone(), api_key).await {
        Ok(data) => Ok(data),
        Err(hydromancer_error) => {
            let mut data = fetch_wallet_details_scoped(address, scope).await?;
            data.warnings
                .push(crate::read_data_provider::fallback_warning(
                    "wallet details",
                    &hydromancer_error,
                ));
            Ok(data)
        }
    }
}

async fn fetch_wallet_details_scoped_hydromancer(
    address: String,
    scope: AccountDataFetchScope,
    api_key: Zeroizing<String>,
) -> Result<WalletDetailsData, String> {
    let portfolio_fut =
        fetch_hydromancer_portfolio_state(address.clone(), scope.clone(), api_key.clone());
    let orders_fut = fetch_hydromancer_frontend_open_orders_scoped(
        address.clone(),
        scope.clone(),
        api_key.clone(),
    );
    let (portfolio, orders_result) = futures::future::join(portfolio_fut, orders_fut).await;
    let portfolio = portfolio?;
    let (clearinghouse, clearinghouses_by_dex, _) = portfolio.clearinghouses_for_scope(&scope)?;
    let spot = portfolio.spot_clearinghouse()?;
    let mut positions = Vec::new();
    for (dex, clearinghouse) in &clearinghouses_by_dex {
        positions.extend(
            clearinghouse
                .asset_positions
                .iter()
                .cloned()
                .map(|asset_position| WalletPositionDetail {
                    dex: dex.clone(),
                    asset_position,
                }),
        );
    }
    let mut warnings = Vec::new();
    let fills =
        fetch_hydromancer_user_fills_if_needed(address, api_key, &spot, &mut warnings).await;
    let orders = match orders_result {
        Ok(orders) => orders,
        Err(error) => {
            warnings.push(error);
            Vec::new()
        }
    };

    let open_orders = orders
        .iter()
        .cloned()
        .map(|order| WalletOpenOrderDetail {
            dex: order_detail_dex(&order),
            order,
        })
        .collect();

    Ok(WalletDetailsData {
        clearinghouse,
        spot,
        positions,
        open_orders,
        fills,
        warnings,
        fetched_at_ms: now_ms(),
    })
}

fn order_detail_dex(order: &OpenOrder) -> String {
    let Some((dex, _)) = order.coin.split_once(':') else {
        return String::new();
    };
    if HIP3_DEXES.iter().any(|known| known == &dex) {
        dex.to_string()
    } else {
        String::new()
    }
}

async fn fetch_wallet_user_fills_if_needed(
    address: &str,
    spot: &SpotClearinghouseState,
    warnings: &mut Vec<String>,
) -> Vec<UserFill> {
    if !spot_state_needs_fill_cost_basis(spot) {
        return Vec::new();
    }

    let fills_resp = crate::api::CLIENT
        .post(API_URL)
        .json(&serde_json::json!({"type": "userFills", "user": address}))
        .send()
        .await;
    best_effort_response_vec("userFills", fills_resp, warnings).await
}

async fn fetch_hydromancer_user_fills_if_needed(
    address: String,
    api_key: Zeroizing<String>,
    spot: &SpotClearinghouseState,
    warnings: &mut Vec<String>,
) -> Vec<UserFill> {
    if !spot_state_needs_fill_cost_basis(spot) {
        return Vec::new();
    }

    match fetch_hydromancer_user_fills(address, api_key).await {
        Ok(fills) => fills,
        Err(error) => {
            warnings.push(error);
            Vec::new()
        }
    }
}

fn spot_state_needs_fill_cost_basis(spot: &SpotClearinghouseState) -> bool {
    spot.balances.iter().any(spot_balance_needs_fill_cost_basis)
}

fn spot_balance_needs_fill_cost_basis(balance: &crate::account::SpotBalance) -> bool {
    if balance.coin.starts_with('+') || spot_balance_coin_is_stable(&balance.coin) {
        return false;
    }

    let total = parse_finite_number(&balance.total).unwrap_or(0.0).abs();
    if total <= 1e-12 {
        return false;
    }

    parse_finite_number(&balance.entry_ntl).unwrap_or(0.0).abs() <= 1e-12
}

fn spot_balance_coin_is_stable(coin: &str) -> bool {
    matches!(coin, "USDC" | "USDE" | "USDT0" | "USDH")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::SpotBalance;

    fn spot_state(balances: Vec<SpotBalance>) -> SpotClearinghouseState {
        SpotClearinghouseState {
            balances,
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        }
    }

    fn balance(coin: &str, total: &str, entry_ntl: &str) -> SpotBalance {
        SpotBalance {
            coin: coin.to_string(),
            token: None,
            total: total.to_string(),
            hold: "0".to_string(),
            entry_ntl: entry_ntl.to_string(),
            supplied: None,
        }
    }

    #[test]
    fn spot_fill_cost_basis_fetch_is_needed_only_for_missing_spot_entry_notional() {
        assert!(spot_state_needs_fill_cost_basis(&spot_state(vec![
            balance("UBTC", "1", "0")
        ])));

        for balance in [
            balance("UBTC", "1", "100"),
            balance("UBTC", "0", "0"),
            balance("USDC", "100", "0"),
            balance("+650", "1", "0"),
        ] {
            assert!(!spot_state_needs_fill_cost_basis(&spot_state(vec![
                balance
            ])));
        }
    }
}

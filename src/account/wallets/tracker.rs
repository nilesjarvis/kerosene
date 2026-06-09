use super::super::{
    AccountDataFetchScope, ClearinghouseState, HydromancerPortfolioState, OpenOrder,
    WalletTrackerSnapshot, fetch_hydromancer_frontend_open_orders_scoped,
    fetch_hydromancer_portfolio_states,
};
use crate::api::API_URL;

use serde_json::Value;

mod hip3;
mod open_orders;
mod snapshot;
mod spot_fallback;

use hip3::append_hip3_margin_and_positions;
use open_orders::fetch_wallet_tracker_open_order_count_scoped;
use snapshot::{build_wallet_tracker_snapshot, parse_tracker_number};
use spot_fallback::apply_spot_equity_fallback;

/// Fetch a low-cost account snapshot for the wallet tracker.
///
/// This intentionally excludes `openOrders`: those requests have much higher
/// rate-limit weight and are refreshed by a separate slow/manual lane.
pub async fn fetch_wallet_tracker_snapshot_scoped(
    address: String,
    scope: AccountDataFetchScope,
) -> Result<WalletTrackerSnapshot, String> {
    let client = crate::api::CLIENT.clone();

    let response = client
        .post(API_URL)
        .json(&serde_json::json!({"type": "clearinghouseState", "user": address}))
        .send()
        .await
        .map_err(|e| format!("clearinghouseState request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "clearinghouseState request failed with HTTP {}",
            response.status()
        ));
    }

    let raw: Value = response
        .json()
        .await
        .map_err(|e| format!("clearinghouseState parse failed: {e}"))?;

    if let Some(err) = raw.get("error").and_then(|v| v.as_str()) {
        return Err(format!("clearinghouseState error: {err}"));
    }

    let main_clearinghouse = serde_json::from_value::<ClearinghouseState>(raw.clone()).ok();
    let mut asset_positions = main_clearinghouse
        .as_ref()
        .map(|ch| ch.asset_positions.clone())
        .unwrap_or_default();
    let mut margin_used = main_clearinghouse
        .as_ref()
        .and_then(|ch| parse_tracker_number(&ch.margin_summary.total_margin_used));

    let mut equity = raw
        .get("marginSummary")
        .and_then(|v| v.get("accountValue"))
        .and_then(|v| v.as_str())
        .and_then(parse_tracker_number);

    let mut withdrawable = raw
        .get("withdrawable")
        .and_then(|v| v.as_str())
        .and_then(parse_tracker_number);

    apply_spot_equity_fallback(&client, &address, &mut equity, &mut withdrawable).await?;
    append_hip3_margin_and_positions(
        &client,
        &address,
        &scope,
        &mut margin_used,
        &mut asset_positions,
    )
    .await;

    Ok(build_wallet_tracker_snapshot(
        equity,
        withdrawable,
        margin_used,
        asset_positions,
    ))
}

pub async fn fetch_wallet_tracker_snapshot_scoped_with_provider(
    address: String,
    scope: AccountDataFetchScope,
    provider: crate::config::ReadDataProvider,
    hydromancer_api_key: String,
) -> Result<WalletTrackerSnapshot, String> {
    if provider != crate::config::ReadDataProvider::Hydromancer {
        return fetch_wallet_tracker_snapshot_scoped(address, scope).await;
    }

    let api_key = hydromancer_api_key.trim().to_string();
    if api_key.is_empty() {
        return fetch_wallet_tracker_snapshot_scoped(address, scope).await;
    }

    let mut results = fetch_wallet_tracker_snapshots_scoped_with_provider(
        vec![address.clone()],
        scope.clone(),
        provider,
        api_key,
    )
    .await;
    match results.pop() {
        Some((_, Ok(snapshot))) => Ok(snapshot),
        Some((_, Err(hydromancer_error))) => {
            fetch_wallet_tracker_snapshot_scoped(address, scope)
                .await
                .map_err(|fallback_error| {
                    format!(
                        "Hydromancer wallet snapshot failed: {hydromancer_error}; Hyperliquid fallback failed: {fallback_error}"
                    )
                })
        }
        None => fetch_wallet_tracker_snapshot_scoped(address, scope).await,
    }
}

pub async fn fetch_wallet_tracker_snapshots_scoped_with_provider(
    addresses: Vec<String>,
    scope: AccountDataFetchScope,
    provider: crate::config::ReadDataProvider,
    hydromancer_api_key: String,
) -> Vec<(String, Result<WalletTrackerSnapshot, String>)> {
    if provider != crate::config::ReadDataProvider::Hydromancer
        || hydromancer_api_key.trim().is_empty()
    {
        return futures::future::join_all(addresses.into_iter().map(|address| {
            let scope = scope.clone();
            async move {
                let result = fetch_wallet_tracker_snapshot_scoped(address.clone(), scope).await;
                (address, result)
            }
        }))
        .await;
    }

    fetch_hydromancer_portfolio_states(addresses, scope.clone(), hydromancer_api_key)
        .await
        .into_iter()
        .map(|(address, result)| {
            let result = result
                .and_then(|portfolio| wallet_tracker_snapshot_from_portfolio(portfolio, &scope));
            (address, result)
        })
        .collect()
}

pub async fn fetch_wallet_tracker_open_order_count_scoped_with_provider(
    address: String,
    scope: AccountDataFetchScope,
    provider: crate::config::ReadDataProvider,
    hydromancer_api_key: String,
) -> Result<usize, String> {
    if provider != crate::config::ReadDataProvider::Hydromancer {
        return fetch_wallet_tracker_open_order_count_scoped(address, scope).await;
    }

    let api_key = hydromancer_api_key.trim().to_string();
    if api_key.is_empty() {
        return fetch_wallet_tracker_open_order_count_scoped(address, scope).await;
    }

    match fetch_hydromancer_frontend_open_orders_scoped(address.clone(), scope.clone(), api_key).await
    {
        Ok(orders) => Ok(orders.into_iter().filter(order_has_size).count()),
        Err(hydromancer_error) => fetch_wallet_tracker_open_order_count_scoped(address, scope)
            .await
            .map_err(|fallback_error| {
                format!(
                    "Hydromancer wallet orders failed: {hydromancer_error}; Hyperliquid fallback failed: {fallback_error}"
                )
            }),
    }
}

fn order_has_size(order: &OpenOrder) -> bool {
    parse_tracker_number(&order.sz)
        .is_some_and(|size| size.is_finite() && size.abs() > f64::EPSILON)
}

fn wallet_tracker_snapshot_from_portfolio(
    portfolio: HydromancerPortfolioState,
    scope: &AccountDataFetchScope,
) -> Result<WalletTrackerSnapshot, String> {
    let (clearinghouse, _, hip3_states) = portfolio.clearinghouses_for_scope(scope)?;
    let equity = parse_tracker_number(&clearinghouse.margin_summary.account_value);
    let withdrawable = parse_tracker_number(&clearinghouse.withdrawable);
    let mut margin_used = parse_tracker_number(&clearinghouse.margin_summary.total_margin_used);
    let mut asset_positions = clearinghouse.asset_positions;
    for hip3_state in hip3_states {
        crate::helpers::add_optional_f64(
            &mut margin_used,
            parse_tracker_number(&hip3_state.margin_summary.total_margin_used),
        );
        asset_positions.extend(hip3_state.asset_positions);
    }
    Ok(build_wallet_tracker_snapshot(
        equity,
        withdrawable,
        margin_used,
        asset_positions,
    ))
}

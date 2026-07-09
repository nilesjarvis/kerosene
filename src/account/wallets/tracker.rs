use super::super::{
    AccountDataFetchScope, AssetPosition, ClearinghouseState, HydromancerPortfolioState, OpenOrder,
    SpotClearinghouseState, WalletTrackerSnapshot, fetch_hydromancer_frontend_open_orders_scoped,
    fetch_hydromancer_portfolio_states,
};
use crate::api::API_URL;

use serde_json::Value;
use zeroize::Zeroizing;

mod hip3;
mod open_orders;
mod snapshot;
mod spot_fallback;

#[cfg(test)]
mod tests;

use hip3::append_hip3_margin_and_positions;
use open_orders::fetch_wallet_tracker_open_order_count_scoped;
use snapshot::{build_wallet_tracker_snapshot, parse_tracker_number};
use spot_fallback::{
    apply_spot_equity_fallback, fetch_spot_fallback_mids, merge_spot_equity_fallback,
    spot_equity_fallback_from_state,
};

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

    // Best-effort like the HIP-3 pass below: a transient failure of the
    // auxiliary spot request must not discard the perp snapshot in hand.
    let valuation_warning =
        apply_spot_equity_fallback(&client, &address, &mut equity, &mut withdrawable).await;
    append_hip3_margin_and_positions(
        &client,
        &address,
        &scope,
        &mut margin_used,
        &mut asset_positions,
    )
    .await;

    let mut snapshot =
        build_wallet_tracker_snapshot(equity, withdrawable, margin_used, asset_positions);
    snapshot.valuation_warning =
        valuation_warning.map(|warning| crate::helpers::redact_sensitive_response_text(&warning));
    Ok(snapshot)
}

pub async fn fetch_wallet_tracker_snapshot_scoped_with_provider(
    address: String,
    scope: AccountDataFetchScope,
    provider: crate::config::ReadDataProvider,
    hydromancer_api_key: Zeroizing<String>,
) -> Result<WalletTrackerSnapshot, String> {
    if provider != crate::config::ReadDataProvider::Hydromancer {
        return fetch_wallet_tracker_snapshot_scoped(address, scope).await;
    }

    let api_key = Zeroizing::new(hydromancer_api_key.trim().to_string());
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
    hydromancer_api_key: Zeroizing<String>,
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

    let values: Vec<(String, Result<PortfolioTrackerValues, String>)> =
        fetch_hydromancer_portfolio_states(addresses, scope.clone(), hydromancer_api_key)
            .await
            .into_iter()
            .map(|(address, result)| {
                let result = result
                    .and_then(|portfolio| wallet_tracker_values_from_portfolio(portfolio, &scope));
                (address, result)
            })
            .collect();

    // Portfolio-margin wallets hold their equity as spot balances, so the
    // perp clearinghouse reports ~0 equity/withdrawable. Price those balances
    // with a single best-effort allMids fetch per batch, mirroring the
    // direct-Hyperliquid spot fallback.
    let needs_mids = values.iter().any(|(_, result)| {
        result
            .as_ref()
            .is_ok_and(|values| values.spot_fallback.is_some())
    });
    let mids = if needs_mids {
        fetch_spot_fallback_mids(&crate::api::CLIENT.clone()).await
    } else {
        Err("no portfolio-margin wallets in batch".to_string())
    };

    values
        .into_iter()
        .map(|(address, result)| {
            let result = result.map(|mut values| {
                if let Some(spot) = values.spot_fallback.take() {
                    if let Err(error) = &mids {
                        values.valuation_warning = Some(format!(
                            "Portfolio-margin spot valuation unavailable: {error}"
                        ));
                    }
                    let outcome = mids
                        .as_ref()
                        .map(|mids| spot_equity_fallback_from_state(&spot, mids))
                        .map_err(String::clone);
                    if outcome.as_ref().is_ok_and(|fallback| {
                        fallback
                            .as_ref()
                            .is_some_and(|fallback| fallback.equity.is_none())
                    }) {
                        values.valuation_warning = Some(
                            "Portfolio-margin spot valuation is incomplete because one or more balances have no live mark"
                                .to_string(),
                        );
                    }
                    merge_spot_equity_fallback(
                        outcome,
                        &mut values.equity,
                        &mut values.withdrawable,
                    );
                }
                values.into_snapshot()
            });
            (address, result)
        })
        .collect()
}

pub async fn fetch_wallet_tracker_open_order_count_scoped_with_provider(
    address: String,
    scope: AccountDataFetchScope,
    provider: crate::config::ReadDataProvider,
    hydromancer_api_key: Zeroizing<String>,
) -> Result<usize, String> {
    if provider != crate::config::ReadDataProvider::Hydromancer {
        return fetch_wallet_tracker_open_order_count_scoped(address, scope).await;
    }

    let api_key = Zeroizing::new(hydromancer_api_key.trim().to_string());
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

/// Snapshot inputs extracted from a Hydromancer portfolio state, with the
/// portfolio-margin spot state retained for the batch-level equity fallback.
struct PortfolioTrackerValues {
    equity: Option<f64>,
    withdrawable: Option<f64>,
    margin_used: Option<f64>,
    asset_positions: Vec<AssetPosition>,
    /// `Some` for portfolio-margin wallets; spot state is authoritative even
    /// when an individual perp-dex response contains a small positive value.
    spot_fallback: Option<SpotClearinghouseState>,
    valuation_warning: Option<String>,
}

impl PortfolioTrackerValues {
    fn into_snapshot(self) -> WalletTrackerSnapshot {
        let mut snapshot = build_wallet_tracker_snapshot(
            self.equity,
            self.withdrawable,
            self.margin_used,
            self.asset_positions,
        );
        snapshot.valuation_warning = self
            .valuation_warning
            .map(|warning| crate::helpers::redact_sensitive_response_text(&warning));
        snapshot
    }
}

fn wallet_tracker_values_from_portfolio(
    portfolio: HydromancerPortfolioState,
    scope: &AccountDataFetchScope,
) -> Result<PortfolioTrackerValues, String> {
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
    let spot_fallback = portfolio
        .spot_clearinghouse()
        .ok()
        .filter(|spot| spot.portfolio_margin_enabled);
    Ok(PortfolioTrackerValues {
        equity,
        withdrawable,
        margin_used,
        asset_positions,
        spot_fallback,
        valuation_warning: None,
    })
}

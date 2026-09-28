mod portfolio;

pub(crate) use self::portfolio::{HydromancerPortfolioState, hydromancer_portfolio_chunk_size};
use self::portfolio::{
    merge_native_and_dex_portfolio_states, parse_batch_portfolio_states, parse_portfolio_state,
};

use super::super::merge::{merge_hip3_open_orders, merge_hip3_positions};
use super::funding_history_start_ms;
use super::responses::{
    fee_rates_from_response, funding_history_from_response, hip3_open_orders_from_response,
};
use super::{frontend_open_orders_payload, user_fills_payload};
use crate::account::{
    AccountData, AccountDataCompleteness, AccountDataFetchScope, AccountDataSection, HIP3_DEXES,
    OpenOrder, UserFill, normalize_dex_open_order_coins,
};
use crate::api::CLIENT;
use crate::app_time::now_ms;
use crate::config::ReadDataProvider;
use crate::helpers::sensitive_response_excerpt;
use crate::hydromancer_api::HYDROMANCER_API_URL;
use crate::network_activity::HttpRequestExt as _;

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Hydromancer Account Fetches
// ---------------------------------------------------------------------------

pub(super) async fn fetch_account_data_scoped_with_provider(
    address: String,
    scope: AccountDataFetchScope,
    provider: ReadDataProvider,
    hydromancer_api_key: Zeroizing<String>,
) -> Result<AccountData, String> {
    if provider != ReadDataProvider::Hydromancer {
        return super::fetch_account_data_scoped(address, scope).await;
    }

    let api_key = Zeroizing::new(hydromancer_api_key.trim().to_string());
    if api_key.is_empty() {
        let mut data = super::fetch_account_data_scoped(address, scope).await?;
        // The Hyperliquid fallback returns a usable positions snapshot for the
        // fetched scope; mark it degraded (not incomplete) so the warning still
        // surfaces while close/NUKE controls stay enabled. If the inner fetch
        // itself dropped positions (e.g. a HIP-3 failure), it already cleared
        // `positions_actionable` and the degrade leaves that block in place.
        data.completeness.mark_degraded(
            AccountDataSection::Positions,
            "Hydromancer API key missing; used Hyperliquid fallback",
        );
        return Ok(data);
    }

    match fetch_account_data_scoped_hydromancer(address.clone(), scope.clone(), api_key).await {
        Ok(data) => Ok(data),
        Err(error) => {
            let mut data = super::fetch_account_data_scoped(address, scope).await?;
            data.completeness.mark_degraded(
                AccountDataSection::Positions,
                crate::read_data_provider::fallback_warning("account refresh", &error),
            );
            Ok(data)
        }
    }
}

async fn fetch_account_data_scoped_hydromancer(
    address: String,
    scope: AccountDataFetchScope,
    api_key: Zeroizing<String>,
) -> Result<AccountData, String> {
    let request_weight_estimate = scope.estimated_info_weight();
    let portfolio_fut =
        fetch_hydromancer_portfolio_state(address.clone(), scope.clone(), api_key.clone());

    let fetch_main_orders = scope.fetches_main_open_orders();
    let main_orders_address = address.clone();
    let main_orders_fut = async {
        if fetch_main_orders {
            Some(
                send_hydromancer_info(
                    frontend_open_orders_payload(&main_orders_address, None),
                    api_key.as_str(),
                )
                .await,
            )
        } else {
            None
        }
    };
    let fills_fut = send_hydromancer_info(user_fills_payload(&address), api_key.as_str());
    let funding_fut = send_hydromancer_info(
        serde_json::json!({
            "type": "userFunding",
            "user": address.clone(),
            "startTime": funding_history_start_ms()
        }),
        api_key.as_str(),
    );
    let fees_fut = send_hydromancer_info(
        serde_json::json!({"type": "userFees", "user": address.clone()}),
        api_key.as_str(),
    );

    let hip3_dexes = scope.hip3_dexes(HIP3_DEXES);
    let hip3_order_futs = hip3_dexes.iter().map(|dex| {
        let api_key = api_key.clone();
        let address = address.clone();
        let dex = dex.clone();
        async move {
            (
                dex.clone(),
                send_hydromancer_info(
                    frontend_open_orders_payload(&address, Some(&dex)),
                    api_key.as_str(),
                )
                .await,
            )
        }
    });

    let main_fut = futures::future::join5(
        portfolio_fut,
        main_orders_fut,
        fills_fut,
        funding_fut,
        fees_fut,
    );
    let (
        (portfolio_raw, main_orders_resp, fills_resp, funding_resp, fees_resp),
        hip3_order_results,
    ) = futures::future::join(main_fut, futures::future::join_all(hip3_order_futs)).await;

    let portfolio = portfolio_raw?;
    let mut completeness = AccountDataCompleteness::default();
    let account_abstraction = portfolio.account_abstraction();
    let spot = portfolio.spot_clearinghouse()?;
    let (clearinghouse, clearinghouses_by_dex, hip3_states) =
        portfolio.clearinghouses_for_scope(&scope)?;

    let mut bootstrap_warnings = Vec::new();
    let mut main_open_orders_fetched = false;
    let open_orders: Vec<OpenOrder> = match main_orders_resp {
        Some(response) => {
            let warning_count = bootstrap_warnings.len();
            let orders =
                hydromancer_response_vec("frontendOpenOrders", response, &mut bootstrap_warnings)
                    .await;
            main_open_orders_fetched = bootstrap_warnings.len() == warning_count;
            orders
        }
        None => Vec::new(),
    };
    let fills: Vec<UserFill> =
        hydromancer_response_vec("userFills", fills_resp, &mut bootstrap_warnings).await;
    super::responses::record_best_effort_section_warnings(&mut completeness, bootstrap_warnings);

    let funding_history = funding_history_from_response(funding_resp, &mut completeness).await;
    let fee_rates = fee_rates_from_response(fees_resp, &mut completeness).await;

    let mut hip3_order_sets = Vec::new();
    let mut hip3_open_orders_fetched = Vec::new();
    for (dex, resp) in hip3_order_results {
        if let Some(orders) = hip3_open_orders_from_response(&dex, resp, &mut completeness).await {
            hip3_open_orders_fetched.push(dex);
            hip3_order_sets.push(orders);
        }
    }

    let fetched_at_ms = now_ms();
    let mut account_data = AccountData {
        fetch_scope: scope,
        request_weight_estimate,
        account_abstraction,
        clearinghouse: merge_hip3_positions(clearinghouse, hip3_states),
        clearinghouses_by_dex,
        spot,
        open_orders: merge_hip3_open_orders(open_orders, hip3_order_sets),
        fills,
        funding_history,
        fee_rates,
        completeness,
        fetched_at_ms,
    };
    if main_open_orders_fetched {
        account_data.mark_open_orders_fetched_at(fetched_at_ms);
    }
    account_data.mark_spot_balances_fetched_at(fetched_at_ms);
    for dex in hip3_open_orders_fetched {
        account_data.mark_open_orders_fetched_at_for_dex(&dex, fetched_at_ms);
    }

    Ok(account_data)
}

pub(crate) async fn fetch_hydromancer_portfolio_state(
    address: String,
    scope: AccountDataFetchScope,
    api_key: Zeroizing<String>,
) -> Result<HydromancerPortfolioState, String> {
    match scope {
        AccountDataFetchScope::AllMarkets { .. } => {
            let raw = post_hydromancer_value(
                "portfolioState",
                serde_json::json!({
                    "type": "portfolioState",
                    "user": address,
                    "dex": "ALL_DEXES"
                }),
                api_key.as_str(),
            )
            .await?;
            parse_portfolio_state(raw)
        }
        AccountDataFetchScope::Hip3Dex { dex } => {
            let dex_payload = dex.clone();
            let native_fut = post_hydromancer_value(
                "portfolioState",
                serde_json::json!({
                    "type": "portfolioState",
                    "user": address,
                }),
                api_key.as_str(),
            );
            let dex_fut = post_hydromancer_value(
                "portfolioState",
                serde_json::json!({
                    "type": "portfolioState",
                    "user": address,
                    "dex": dex_payload,
                }),
                api_key.as_str(),
            );
            let (native_raw, dex_raw) = futures::future::join(native_fut, dex_fut).await;
            merge_native_and_dex_portfolio_states(native_raw?, dex_raw?, &dex)
        }
    }
}

pub(crate) async fn fetch_hydromancer_portfolio_states(
    addresses: Vec<String>,
    scope: AccountDataFetchScope,
    api_key: Zeroizing<String>,
) -> Vec<(String, Result<HydromancerPortfolioState, String>)> {
    if addresses.is_empty() {
        return Vec::new();
    }

    let chunk_size = hydromancer_portfolio_chunk_size(&scope);
    match scope {
        AccountDataFetchScope::AllMarkets { .. } => {
            let states = fetch_hydromancer_batch_portfolio_values(
                &addresses,
                Some("ALL_DEXES"),
                chunk_size,
                api_key.as_str(),
            )
            .await;
            addresses
                .into_iter()
                .map(|address| {
                    let result = states
                        .get(&address_key(&address))
                        .cloned()
                        .unwrap_or_else(|| Err("batchPortfolioStates missing wallet".to_string()))
                        .and_then(parse_portfolio_state);
                    (address, result)
                })
                .collect()
        }
        AccountDataFetchScope::Hip3Dex { dex } => {
            let native_fut = fetch_hydromancer_batch_portfolio_values(
                &addresses,
                None,
                chunk_size,
                api_key.as_str(),
            );
            let dex_fut = fetch_hydromancer_batch_portfolio_values(
                &addresses,
                Some(dex.as_str()),
                chunk_size,
                api_key.as_str(),
            );
            let (native_states, dex_states) = futures::future::join(native_fut, dex_fut).await;
            addresses
                .into_iter()
                .map(|address| {
                    let key = address_key(&address);
                    let result = match (native_states.get(&key), dex_states.get(&key)) {
                        (Some(Ok(native_raw)), Some(Ok(dex_raw))) => {
                            merge_native_and_dex_portfolio_states(
                                native_raw.clone(),
                                dex_raw.clone(),
                                &dex,
                            )
                        }
                        (Some(Err(error)), _) => Err(error.clone()),
                        (_, Some(Err(error))) => Err(error.clone()),
                        (None, _) | (_, None) => {
                            Err("batchPortfolioStates missing wallet".to_string())
                        }
                    };
                    (address, result)
                })
                .collect()
        }
    }
}

pub(crate) async fn fetch_hydromancer_frontend_open_orders_scoped(
    address: String,
    scope: AccountDataFetchScope,
    api_key: Zeroizing<String>,
) -> Result<Vec<OpenOrder>, String> {
    let mut order_futs = Vec::new();
    if scope.fetches_main_open_orders() {
        let order_address = address.clone();
        order_futs.push((
            String::new(),
            post_hydromancer_vec::<OpenOrder>(
                "frontendOpenOrders",
                frontend_open_orders_payload(&order_address, None),
                api_key.clone(),
            ),
        ));
    }

    for dex in scope.hip3_dexes(HIP3_DEXES) {
        let order_address = address.clone();
        order_futs.push((
            dex.clone(),
            post_hydromancer_vec::<OpenOrder>(
                "frontendOpenOrders",
                frontend_open_orders_payload(&order_address, Some(&dex)),
                api_key.clone(),
            ),
        ));
    }

    let mut orders = Vec::new();
    let mut failures = Vec::new();
    for (dex, result) in futures::future::join_all(
        order_futs
            .into_iter()
            .map(|(dex, fut)| async move { (dex, fut.await) }),
    )
    .await
    {
        match result {
            Ok(mut dex_orders) => {
                normalize_dex_open_order_coins(&dex, &mut dex_orders);
                orders.extend(dex_orders);
            }
            Err(error) => failures.push(if dex.is_empty() {
                error
            } else {
                format!("{dex} {error}")
            }),
        }
    }

    if failures.is_empty() {
        Ok(orders)
    } else {
        Err(format!(
            "frontendOpenOrders refresh partially failed: {}",
            failures.join("; ")
        ))
    }
}

pub(crate) async fn fetch_hydromancer_user_fills(
    address: String,
    api_key: Zeroizing<String>,
) -> Result<Vec<UserFill>, String> {
    post_hydromancer_vec("userFills", user_fills_payload(&address), api_key).await
}

async fn send_hydromancer_info(
    payload: Value,
    api_key: &str,
) -> Result<reqwest::Response, reqwest::Error> {
    CLIENT
        .post(HYDROMANCER_API_URL)
        .bearer_auth(api_key.trim())
        .json(&payload)
        .send_observed()
        .await
}

async fn post_hydromancer_value(
    label: &'static str,
    payload: Value,
    api_key: &str,
) -> Result<Value, String> {
    let response = send_hydromancer_info(payload, api_key)
        .await
        .map_err(|e| format!("{label} request failed: {e}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("{label} response read failed: {e}"))?;
    if !status.is_success() {
        let body = sensitive_response_excerpt(&text, 160);
        return if body.is_empty() {
            Err(format!("{label} request failed with HTTP {status}"))
        } else {
            Err(format!("{label} request failed with HTTP {status}: {body}"))
        };
    }

    serde_json::from_str(&text).map_err(|e| format!("{label} parse failed: {e}"))
}

async fn post_hydromancer_vec<T>(
    label: &'static str,
    payload: Value,
    api_key: Zeroizing<String>,
) -> Result<Vec<T>, String>
where
    T: for<'de> Deserialize<'de>,
{
    let value = post_hydromancer_value(label, payload, api_key.as_str()).await?;
    serde_json::from_value(value).map_err(|e| format!("{label} parse failed: {e}"))
}

async fn fetch_hydromancer_batch_portfolio_values(
    addresses: &[String],
    dex: Option<&str>,
    chunk_size: usize,
    api_key: &str,
) -> HashMap<String, Result<Value, String>> {
    let mut results = HashMap::new();
    for chunk in addresses.chunks(chunk_size.max(1)) {
        let mut payload = serde_json::json!({
            "type": "batchPortfolioStates",
            "users": chunk,
        });
        if let Some(dex) = dex
            && let Some(object) = payload.as_object_mut()
        {
            object.insert("dex".to_string(), Value::String(dex.to_string()));
        }

        match post_hydromancer_value("batchPortfolioStates", payload, api_key).await {
            Ok(raw) => match parse_batch_portfolio_states(raw) {
                Ok(batch) => {
                    for (address, state) in batch.successful_states {
                        results.insert(address_key(&address), Ok(state));
                    }
                    for address in batch.failed_wallets {
                        results.insert(
                            address_key(&address),
                            Err("batchPortfolioStates failed for wallet".to_string()),
                        );
                    }
                    for address in chunk {
                        results.entry(address_key(address)).or_insert_with(|| {
                            Err("batchPortfolioStates missing wallet".to_string())
                        });
                    }
                }
                Err(error) => {
                    for address in chunk {
                        results.insert(address_key(address), Err(error.clone()));
                    }
                }
            },
            Err(error) => {
                for address in chunk {
                    results.insert(address_key(address), Err(error.clone()));
                }
            }
        }
    }
    results
}

fn address_key(address: &str) -> String {
    address.to_ascii_lowercase()
}

async fn hydromancer_response_vec<T>(
    label: &'static str,
    response: Result<reqwest::Response, reqwest::Error>,
    warnings: &mut Vec<String>,
) -> Vec<T>
where
    T: for<'de> Deserialize<'de>,
{
    let response = match response {
        Ok(response) => response,
        Err(e) => {
            warnings.push(format!("{label} request failed: {e}"));
            return Vec::new();
        }
    };

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let body = sensitive_response_excerpt(&body, 160);
        if body.is_empty() {
            warnings.push(format!("{label} request failed with HTTP {status}"));
        } else {
            warnings.push(format!("{label} request failed with HTTP {status}: {body}"));
        }
        return Vec::new();
    }

    match response.json::<Vec<T>>().await {
        Ok(items) => items,
        Err(e) => {
            warnings.push(format!("{label} parse failed: {e}"));
            Vec::new()
        }
    }
}

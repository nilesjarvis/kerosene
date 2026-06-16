use super::super::merge::{merge_hip3_open_orders, merge_hip3_positions};
use super::funding_history_start_ms;
use super::responses::{
    fee_rates_from_response, funding_history_from_response, hip3_open_orders_from_response,
};
use super::{frontend_open_orders_payload, user_fills_payload};
use crate::account::{
    AccountAbstractionMode, AccountData, AccountDataCompleteness, AccountDataFetchScope,
    AccountDataSection, ClearinghouseState, HIP3_DEXES, OpenOrder, SpotClearinghouseState,
    UserFill, normalize_dex_asset_position_coins, normalize_dex_open_order_coins,
};
use crate::api::CLIENT;
use crate::app_time::now_ms;
use crate::config::ReadDataProvider;
use crate::helpers::sensitive_response_excerpt;
use crate::hydromancer_api::HYDROMANCER_API_URL;

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use zeroize::Zeroizing;

pub(crate) type PortfolioClearinghouses = (
    ClearinghouseState,
    HashMap<String, ClearinghouseState>,
    Vec<ClearinghouseState>,
);

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
    let open_orders: Vec<OpenOrder> = match main_orders_resp {
        Some(response) => {
            hydromancer_response_vec("frontendOpenOrders", response, &mut bootstrap_warnings).await
        }
        None => Vec::new(),
    };
    let fills: Vec<UserFill> =
        hydromancer_response_vec("userFills", fills_resp, &mut bootstrap_warnings).await;
    super::responses::record_best_effort_section_warnings(&mut completeness, bootstrap_warnings);

    let funding_history = funding_history_from_response(funding_resp, &mut completeness).await;
    let fee_rates = fee_rates_from_response(fees_resp, &mut completeness).await;

    let mut hip3_order_sets = Vec::new();
    for (dex, resp) in hip3_order_results {
        if let Some(orders) = hip3_open_orders_from_response(&dex, resp, &mut completeness).await {
            hip3_order_sets.push(orders);
        }
    }

    Ok(AccountData {
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
        fetched_at_ms: now_ms(),
    })
}

pub(crate) fn hydromancer_portfolio_chunk_size(scope: &AccountDataFetchScope) -> usize {
    match scope {
        AccountDataFetchScope::AllMarkets { .. } => 100,
        AccountDataFetchScope::Hip3Dex { .. } => 500,
    }
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

    match scope {
        AccountDataFetchScope::AllMarkets { .. } => {
            let states = fetch_hydromancer_batch_portfolio_values(
                addresses.clone(),
                Some("ALL_DEXES"),
                100,
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
                addresses.clone(),
                None,
                500,
                api_key.as_str(),
            );
            let dex_fut = fetch_hydromancer_batch_portfolio_values(
                addresses.clone(),
                Some(dex.as_str()),
                500,
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

async fn send_hydromancer_info(
    payload: Value,
    api_key: &str,
) -> Result<reqwest::Response, reqwest::Error> {
    CLIENT
        .clone()
        .post(HYDROMANCER_API_URL)
        .bearer_auth(api_key.trim())
        .json(&payload)
        .send()
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
    addresses: Vec<String>,
    dex: Option<&str>,
    chunk_size: usize,
    api_key: &str,
) -> HashMap<String, Result<Value, String>> {
    let mut results = HashMap::new();
    for chunk in addresses.chunks(chunk_size.max(1)) {
        let users = chunk.to_vec();
        let mut payload = serde_json::json!({
            "type": "batchPortfolioStates",
            "users": users,
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

struct HydromancerBatchPortfolioStates {
    successful_states: Vec<(String, Value)>,
    failed_wallets: Vec<String>,
}

fn parse_batch_portfolio_states(raw: Value) -> Result<HydromancerBatchPortfolioStates, String> {
    let successful_raw = raw
        .get("successful_states")
        .or_else(|| raw.get("successfulStates"))
        .and_then(Value::as_array)
        .ok_or_else(|| "batchPortfolioStates missing successful_states".to_string())?;
    let mut successful_states = Vec::new();
    for item in successful_raw {
        let pair = item
            .as_array()
            .ok_or_else(|| "batchPortfolioStates successful state was not a tuple".to_string())?;
        if pair.len() != 2 {
            return Err("batchPortfolioStates successful state tuple had wrong length".to_string());
        }
        let address = pair[0]
            .as_str()
            .ok_or_else(|| "batchPortfolioStates successful state missing address".to_string())?
            .to_string();
        successful_states.push((address, pair[1].clone()));
    }

    let failed_wallets = raw
        .get("failed_wallets")
        .or_else(|| raw.get("failedWallets"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    Ok(HydromancerBatchPortfolioStates {
        successful_states,
        failed_wallets,
    })
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

#[derive(Debug)]
pub(crate) struct HydromancerPortfolioState {
    clearinghouse_state: Value,
    spot_clearinghouse_state: Value,
    user_abstraction: Value,
}

impl HydromancerPortfolioState {
    pub(crate) fn account_abstraction(&self) -> AccountAbstractionMode {
        self.user_abstraction
            .as_str()
            .map(AccountAbstractionMode::from_api_value)
            .unwrap_or_else(|| AccountAbstractionMode::Unknown(self.user_abstraction.to_string()))
    }

    pub(crate) fn spot_clearinghouse(&self) -> Result<SpotClearinghouseState, String> {
        serde_json::from_value(self.spot_clearinghouse_state.clone())
            .map_err(|e| format!("spotClearinghouseState deserialize failed: {e}"))
    }

    pub(crate) fn clearinghouses_for_scope(
        &self,
        scope: &AccountDataFetchScope,
    ) -> Result<PortfolioClearinghouses, String> {
        if self.clearinghouse_state.get("marginSummary").is_some() {
            let clearinghouse = parse_clearinghouse_state("", self.clearinghouse_state.clone())?;
            let mut clearinghouses_by_dex = HashMap::new();
            clearinghouses_by_dex.insert(String::new(), clearinghouse.clone());
            return Ok((clearinghouse, clearinghouses_by_dex, Vec::new()));
        }

        let states = self
            .clearinghouse_state
            .as_object()
            .ok_or_else(|| "portfolioState clearinghouseState was not an object".to_string())?;
        let native_raw = states
            .get("native")
            .or_else(|| states.get(""))
            .ok_or_else(|| "portfolioState missing native clearinghouseState".to_string())?;
        let native = parse_clearinghouse_state("", native_raw.clone())?;
        let mut clearinghouses_by_dex = HashMap::new();
        clearinghouses_by_dex.insert(String::new(), native.clone());

        let mut hip3_states = Vec::new();
        for dex in scope.hip3_dexes(HIP3_DEXES) {
            let Some(raw) = states.get(&dex) else {
                continue;
            };
            let state = parse_clearinghouse_state(&dex, raw.clone())?;
            clearinghouses_by_dex.insert(dex, state.clone());
            hip3_states.push(state);
        }

        Ok((native, clearinghouses_by_dex, hip3_states))
    }
}

fn parse_portfolio_state(raw: Value) -> Result<HydromancerPortfolioState, String> {
    Ok(HydromancerPortfolioState {
        clearinghouse_state: raw
            .get("clearinghouseState")
            .cloned()
            .ok_or_else(|| "portfolioState missing clearinghouseState".to_string())?,
        spot_clearinghouse_state: raw
            .get("spotClearinghouseState")
            .cloned()
            .ok_or_else(|| "portfolioState missing spotClearinghouseState".to_string())?,
        user_abstraction: raw
            .get("userAbstraction")
            .cloned()
            .unwrap_or_else(|| Value::String("default".to_string())),
    })
}

fn merge_native_and_dex_portfolio_states(
    native_raw: Value,
    dex_raw: Value,
    dex: &str,
) -> Result<HydromancerPortfolioState, String> {
    let native = parse_portfolio_state(native_raw)?;
    let dex_state = parse_portfolio_state(dex_raw)?;
    let mut clearinghouse_state = serde_json::Map::new();
    clearinghouse_state.insert("native".to_string(), native.clearinghouse_state);
    clearinghouse_state.insert(dex.to_string(), dex_state.clearinghouse_state);
    Ok(HydromancerPortfolioState {
        clearinghouse_state: Value::Object(clearinghouse_state),
        spot_clearinghouse_state: native.spot_clearinghouse_state,
        user_abstraction: native.user_abstraction,
    })
}

fn parse_clearinghouse_state(dex: &str, raw: Value) -> Result<ClearinghouseState, String> {
    let mut clearinghouse = serde_json::from_value::<ClearinghouseState>(raw)
        .map_err(|e| format!("{dex} clearinghouseState deserialize failed: {e}"))?;
    normalize_dex_asset_position_coins(dex, &mut clearinghouse.asset_positions);
    Ok(clearinghouse)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portfolio_state_all_dexes_parses_and_normalizes_dex_positions() {
        let raw = serde_json::json!({
            "clearinghouseState": {
                "native": clearinghouse_state_json("BTC"),
                "xyz": clearinghouse_state_json("MSFT")
            },
            "spotClearinghouseState": {
                "balances": [
                    {
                        "coin": "USDC",
                        "token": 0,
                        "total": "10",
                        "hold": "0",
                        "entryNtl": "0"
                    }
                ]
            },
            "userAbstraction": "unifiedAccount"
        });

        let portfolio = parse_portfolio_state(raw).expect("portfolio parses");
        assert_eq!(
            portfolio.account_abstraction(),
            AccountAbstractionMode::UnifiedAccount
        );
        let (native, by_dex, hip3_states) = portfolio
            .clearinghouses_for_scope(&AccountDataFetchScope::hip3_dex("xyz"))
            .expect("clearinghouses parse");

        assert_eq!(native.asset_positions[0].position.coin, "BTC");
        assert_eq!(by_dex["xyz"].asset_positions[0].position.coin, "xyz:MSFT");
        assert_eq!(hip3_states[0].asset_positions[0].position.coin, "xyz:MSFT");
        assert_eq!(
            portfolio
                .spot_clearinghouse()
                .expect("spot parses")
                .balances[0]
                .coin,
            "USDC"
        );
    }

    #[test]
    fn portfolio_state_direct_clearinghouse_shape_parses_native_state() {
        let raw = serde_json::json!({
            "clearinghouseState": clearinghouse_state_json("ETH"),
            "spotClearinghouseState": { "balances": [] },
            "userAbstraction": "default"
        });

        let portfolio = parse_portfolio_state(raw).expect("portfolio parses");
        let (native, by_dex, hip3_states) = portfolio
            .clearinghouses_for_scope(&AccountDataFetchScope::default())
            .expect("clearinghouse parses");

        assert_eq!(native.asset_positions[0].position.coin, "ETH");
        assert_eq!(by_dex.len(), 1);
        assert!(hip3_states.is_empty());
    }

    #[test]
    fn batch_portfolio_states_parses_successes_and_failures() {
        let raw = serde_json::json!({
            "successful_states": [
                [
                    "0x0000000000000000000000000000000000000001",
                    {
                        "clearinghouseState": clearinghouse_state_json("BTC"),
                        "spotClearinghouseState": { "balances": [] },
                        "userAbstraction": "default"
                    }
                ]
            ],
            "failed_wallets": ["0x0000000000000000000000000000000000000002"]
        });

        let batch = parse_batch_portfolio_states(raw).expect("batch parses");

        assert_eq!(batch.successful_states.len(), 1);
        assert_eq!(
            batch.successful_states[0].0,
            "0x0000000000000000000000000000000000000001"
        );
        assert_eq!(
            batch.failed_wallets,
            vec!["0x0000000000000000000000000000000000000002"]
        );
    }

    #[test]
    fn selected_dex_portfolios_merge_without_all_dexes() {
        let native_raw = serde_json::json!({
            "clearinghouseState": clearinghouse_state_json("BTC"),
            "spotClearinghouseState": { "balances": [] },
            "userAbstraction": "default"
        });
        let dex_raw = serde_json::json!({
            "clearinghouseState": clearinghouse_state_json("MSFT"),
            "spotClearinghouseState": { "balances": [] },
            "userAbstraction": "default"
        });

        let portfolio = merge_native_and_dex_portfolio_states(native_raw, dex_raw, "xyz")
            .expect("portfolio merges");
        let (native, by_dex, hip3_states) = portfolio
            .clearinghouses_for_scope(&AccountDataFetchScope::hip3_dex("xyz"))
            .expect("clearinghouses parse");

        assert_eq!(native.asset_positions[0].position.coin, "BTC");
        assert_eq!(by_dex.len(), 2);
        assert_eq!(hip3_states[0].asset_positions[0].position.coin, "xyz:MSFT");
    }

    #[test]
    fn hydromancer_batch_chunk_size_matches_api_limits() {
        assert_eq!(
            hydromancer_portfolio_chunk_size(&AccountDataFetchScope::default()),
            100
        );
        assert_eq!(
            hydromancer_portfolio_chunk_size(&AccountDataFetchScope::hip3_dex("xyz")),
            500
        );
    }

    fn clearinghouse_state_json(coin: &str) -> Value {
        serde_json::json!({
            "marginSummary": {
                "accountValue": "100",
                "totalNtlPos": "10",
                "totalMarginUsed": "5"
            },
            "crossMarginSummary": {
                "accountValue": "100",
                "totalNtlPos": "10",
                "totalMarginUsed": "5"
            },
            "crossMaintenanceMarginUsed": "1",
            "withdrawable": "95",
            "assetPositions": [
                {
                    "position": {
                        "coin": coin,
                        "szi": "1",
                        "entryPx": "10",
                        "positionValue": "10",
                        "unrealizedPnl": "0",
                        "liquidationPx": null,
                        "leverage": {
                            "type": "cross",
                            "value": 3
                        },
                        "marginUsed": "3"
                    }
                }
            ]
        })
    }
}

use super::super::http::{best_effort_response_vec, post_info_json_with_retries};
use super::super::{
    AccountData, AccountDataCompleteness, AccountDataFetchScope, HIP3_DEXES, OpenOrder, UserFill,
};
use super::merge::{merge_hip3_open_orders, merge_hip3_positions};
use crate::api::API_URL;
use crate::app_time::now_ms;
use responses::{
    account_abstraction_from_best_effort_value, clearinghouse_from_required_value,
    fee_rates_from_response, funding_history_from_response, hip3_clearinghouse_from_response,
    hip3_open_orders_from_response, record_best_effort_section_warnings, spot_from_required_value,
};
use serde_json::Value;

use std::collections::HashMap;

mod hydromancer;
mod responses;

#[cfg(test)]
mod tests;

pub(crate) use hydromancer::{
    HydromancerPortfolioState, fetch_hydromancer_frontend_open_orders_scoped,
    fetch_hydromancer_portfolio_state, fetch_hydromancer_portfolio_states,
    hydromancer_portfolio_chunk_size,
};

const FUNDING_HISTORY_LOOKBACK_MS: u64 = 7 * 24 * 60 * 60 * 1000;

fn frontend_open_orders_payload(address: &str, dex: Option<&str>) -> Value {
    let mut payload = serde_json::json!({
        "type": "frontendOpenOrders",
        "user": address,
    });
    if let Some(dex) = dex.filter(|dex| !dex.is_empty())
        && let Some(object) = payload.as_object_mut()
    {
        object.insert("dex".to_string(), serde_json::json!(dex));
    }
    payload
}

fn user_fills_payload(address: &str) -> Value {
    serde_json::json!({
        "type": "userFills",
        "user": address,
    })
}

// ---------------------------------------------------------------------------
// Account Data Fetches
// ---------------------------------------------------------------------------

/// Fetch account data for a user address, scoped to the visible market universe.
/// All HTTP requests are fired concurrently to minimize total latency.
pub async fn fetch_account_data_scoped(
    address: String,
    scope: AccountDataFetchScope,
) -> Result<AccountData, String> {
    let client = crate::api::CLIENT.clone();
    let request_weight_estimate = scope.estimated_info_weight();

    // Main dex: clearinghouse, spot, orders, fills, funding
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
    let abstraction_fut = post_info_json_with_retries(
        client.clone(),
        "userAbstraction",
        serde_json::json!({"type": "userAbstraction", "user": address}),
    );
    let fetch_main_orders = scope.fetches_main_open_orders();
    let orders_fut = async {
        if fetch_main_orders {
            Some(
                client
                    .post(API_URL)
                    .json(&frontend_open_orders_payload(&address, None))
                    .send()
                    .await,
            )
        } else {
            None
        }
    };
    let fills_fut = client
        .post(API_URL)
        .json(&user_fills_payload(&address))
        .send();
    let funding_fut = client
        .post(API_URL)
        .json(&serde_json::json!({
            "type": "userFunding",
            "user": address,
            "startTime": funding_history_start_ms()
        }))
        .send();

    // User fee rates (fired in parallel with everything else)
    let fees_fut = client
        .post(API_URL)
        .json(&serde_json::json!({"type": "userFees", "user": address}))
        .send();

    // HIP-3 dexes: clearinghouse + orders for each (fired in parallel)
    let hip3_dexes = scope.hip3_dexes(HIP3_DEXES);
    let mut hip3_ch_futs = Vec::new();
    let mut hip3_ord_futs = Vec::new();
    for dex in &hip3_dexes {
        hip3_ch_futs.push(
            client
                .post(API_URL)
                .json(&serde_json::json!({
                    "type": "clearinghouseState",
                    "user": address,
                    "dex": dex
                }))
                .send(),
        );
        hip3_ord_futs.push(
            client
                .post(API_URL)
                .json(&frontend_open_orders_payload(&address, Some(dex)))
                .send(),
        );
    }

    // Fire every independent request together so the runtime polls them
    // concurrently. The earlier two-wave layout (main `join5`, then a 50ms
    // sleep, then HIP-3 + fees in a second join) was effectively
    // serializing: `reqwest::send()` returns a lazy future that doesn't
    // hit the network until polled, so the comments claiming HIP-3 and
    // userFees "fire in parallel" did not match runtime behavior. Now all
    // requests are polled in the same wave. The HIP-3 portion is scoped to
    // the visible exchange when the terminal is in a single-exchange universe.
    let main_fut = futures::future::join(
        futures::future::join5(ch_fut, spot_fut, orders_fut, fills_fut, funding_fut),
        abstraction_fut,
    );
    let hip3_ch_join = futures::future::join_all(hip3_ch_futs);
    let hip3_ord_join = futures::future::join_all(hip3_ord_futs);
    let (
        ((ch_resp, spot_resp, orders_resp, fills_resp, funding_resp), abstraction_resp),
        hip3_ch_results,
        hip3_ord_results,
        fees_resp,
    ) = futures::future::join4(main_fut, hip3_ch_join, hip3_ord_join, fees_fut).await;

    let clearinghouse = clearinghouse_from_required_value(ch_resp?)?;
    let spot = spot_from_required_value(spot_resp?)?;

    let mut completeness = AccountDataCompleteness::default();
    let account_abstraction =
        account_abstraction_from_best_effort_value(abstraction_resp, &spot, &mut completeness);
    let mut bootstrap_warnings = Vec::new();
    let open_orders: Vec<OpenOrder> = match orders_resp {
        Some(orders_resp) => {
            best_effort_response_vec("frontendOpenOrders", orders_resp, &mut bootstrap_warnings)
                .await
        }
        None => Vec::new(),
    };
    let fills: Vec<UserFill> =
        best_effort_response_vec("userFills", fills_resp, &mut bootstrap_warnings).await;
    record_best_effort_section_warnings(&mut completeness, bootstrap_warnings);

    let funding_history = funding_history_from_response(funding_resp, &mut completeness).await;

    let fee_rates = fee_rates_from_response(fees_resp, &mut completeness).await;

    let mut clearinghouses_by_dex = HashMap::new();
    clearinghouses_by_dex.insert(String::new(), clearinghouse.clone());

    let mut hip3_states = Vec::new();
    for (dex, resp) in hip3_dexes.iter().zip(hip3_ch_results) {
        if let Some(ch) = hip3_clearinghouse_from_response(dex, resp, &mut completeness).await {
            clearinghouses_by_dex.insert(dex.to_string(), ch.clone());
            hip3_states.push(ch);
        }
    }

    let mut hip3_order_sets = Vec::new();
    for (dex, resp) in hip3_dexes.iter().zip(hip3_ord_results) {
        if let Some(orders) = hip3_open_orders_from_response(dex, resp, &mut completeness).await {
            hip3_order_sets.push(orders);
        }
    }

    let merged_clearinghouse = merge_hip3_positions(clearinghouse, hip3_states);
    let open_orders = merge_hip3_open_orders(open_orders, hip3_order_sets);

    Ok(AccountData {
        fetch_scope: scope,
        request_weight_estimate,
        account_abstraction,
        clearinghouse: merged_clearinghouse,
        clearinghouses_by_dex,
        spot,
        open_orders,
        fills,
        funding_history,
        fee_rates,
        completeness,
        fetched_at_ms: now_ms(),
    })
}

pub async fn fetch_account_data_scoped_with_provider(
    address: String,
    scope: AccountDataFetchScope,
    provider: crate::config::ReadDataProvider,
    hydromancer_api_key: zeroize::Zeroizing<String>,
) -> Result<AccountData, String> {
    hydromancer::fetch_account_data_scoped_with_provider(
        address,
        scope,
        provider,
        hydromancer_api_key,
    )
    .await
}

fn funding_history_start_ms() -> u64 {
    funding_history_start_ms_from(now_ms())
}

fn funding_history_start_ms_from(now_ms: u64) -> u64 {
    now_ms.saturating_sub(FUNDING_HISTORY_LOOKBACK_MS)
}

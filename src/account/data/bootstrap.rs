use super::super::http::{best_effort_response_vec, post_info_json_with_retries};
use super::super::{
    AccountAbstractionMode, AccountData, AccountDataCompleteness, AccountDataFetchScope,
    AccountDataSection, ClearinghouseState, FundingEntry, HIP3_DEXES, OpenOrder,
    SpotClearinghouseState, UserFeeRates, UserFill, normalize_dex_asset_position_coins,
    normalize_dex_open_order_coins,
};
use super::fees::user_fee_rates_from_value;
use super::merge::{merge_hip3_open_orders, merge_hip3_positions};
use crate::api::API_URL;

use serde_json::Value;
use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Account Data Fetches
// ---------------------------------------------------------------------------

fn fee_rates_from_best_effort_value(
    raw: Result<Value, String>,
    completeness: &mut AccountDataCompleteness,
) -> UserFeeRates {
    match raw {
        Ok(raw) => user_fee_rates_from_value(&raw),
        Err(error) => {
            completeness.mark_incomplete(AccountDataSection::Fees, error);
            UserFeeRates::default()
        }
    }
}

fn account_abstraction_from_best_effort_value(
    raw: Result<Value, String>,
    spot: &SpotClearinghouseState,
    completeness: &mut AccountDataCompleteness,
) -> AccountAbstractionMode {
    if spot.portfolio_margin_enabled {
        return AccountAbstractionMode::PortfolioMargin;
    }

    match raw {
        Ok(raw) => raw
            .as_str()
            .map(AccountAbstractionMode::from_api_value)
            .unwrap_or_else(|| AccountAbstractionMode::Unknown(raw.to_string())),
        Err(error) => {
            completeness.mark_incomplete(AccountDataSection::Positions, error);
            AccountAbstractionMode::Unknown("unavailable".to_string())
        }
    }
}

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
                    .json(&serde_json::json!({"type": "frontendOpenOrders", "user": address}))
                    .send()
                    .await,
            )
        } else {
            None
        }
    };
    let fills_fut = client
        .post(API_URL)
        .json(&serde_json::json!({"type": "userFills", "user": address}))
        .send();
    // Funding history: last 7 days
    let seven_days_ms = 7 * 24 * 60 * 60 * 1000_u64;
    let funding_start = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
        - seven_days_ms;
    let funding_fut = client
        .post(API_URL)
        .json(&serde_json::json!({
            "type": "userFunding",
            "user": address,
            "startTime": funding_start
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
                .json(&serde_json::json!({
                    "type": "frontendOpenOrders",
                    "user": address,
                    "dex": dex
                }))
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

    let ch_raw = ch_resp?;
    let clearinghouse: ClearinghouseState =
        serde_json::from_value(ch_raw.clone()).map_err(|e| {
            format!(
                "clearinghouseState deserialize failed: {e} | JSON: {}",
                ch_raw.to_string().chars().take(200).collect::<String>()
            )
        })?;

    let spot_raw = spot_resp?;
    let spot: SpotClearinghouseState = serde_json::from_value(spot_raw)
        .map_err(|e| format!("spotClearinghouseState deserialize failed: {e}"))?;

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
    for warning in bootstrap_warnings {
        if warning.starts_with("frontendOpenOrders") {
            completeness.mark_incomplete(AccountDataSection::OpenOrders, warning);
        } else if warning.starts_with("userFills") {
            completeness.mark_incomplete(AccountDataSection::Fills, warning);
        } else {
            completeness.mark_incomplete(AccountDataSection::Positions, warning);
        }
    }

    // Funding history is best-effort; don't fail the whole fetch if this fails.
    let funding_history: Vec<FundingEntry> = match funding_resp {
        Ok(resp) if resp.status().is_success() => match resp.json().await {
            Ok(entries) => entries,
            Err(e) => {
                completeness.mark_incomplete(
                    AccountDataSection::Funding,
                    format!("userFunding parse failed: {e}"),
                );
                Vec::new()
            }
        },
        Ok(resp) => {
            completeness.mark_incomplete(
                AccountDataSection::Funding,
                format!("userFunding request failed with HTTP {}", resp.status()),
            );
            Vec::new()
        }
        Err(e) => {
            completeness.mark_incomplete(
                AccountDataSection::Funding,
                format!("userFunding request failed: {e}"),
            );
            Vec::new()
        }
    };

    // Fee rates are best-effort; parse from raw Value because this response
    // contains mixed-type fields that can trip up strict typed deserialization.
    let fee_rates: UserFeeRates = match fees_resp {
        Ok(resp) => {
            if resp.status().is_success() {
                fee_rates_from_best_effort_value(
                    resp.json::<Value>()
                        .await
                        .map_err(|e| format!("userFees parse failed: {e}")),
                    &mut completeness,
                )
            } else {
                completeness.mark_incomplete(
                    AccountDataSection::Fees,
                    format!("userFees request failed with HTTP {}", resp.status()),
                );
                UserFeeRates::default()
            }
        }
        Err(e) => {
            completeness.mark_incomplete(
                AccountDataSection::Fees,
                format!("userFees request failed: {e}"),
            );
            UserFeeRates::default()
        }
    };

    let mut clearinghouses_by_dex = HashMap::new();
    clearinghouses_by_dex.insert(String::new(), clearinghouse.clone());

    let mut hip3_states = Vec::new();
    for (dex, resp) in hip3_dexes.iter().zip(hip3_ch_results) {
        match resp {
            Ok(response) if response.status().is_success() => {
                match response.json::<Value>().await {
                    Ok(raw) => match serde_json::from_value::<ClearinghouseState>(raw) {
                        Ok(mut ch) => {
                            normalize_dex_asset_position_coins(dex, &mut ch.asset_positions);
                            clearinghouses_by_dex.insert(dex.to_string(), ch.clone());
                            hip3_states.push(ch);
                        }
                        Err(e) => completeness.mark_incomplete(
                            AccountDataSection::Positions,
                            format!("HIP-3 clearinghouseState parse failed: {e}"),
                        ),
                    },
                    Err(e) => completeness.mark_incomplete(
                        AccountDataSection::Positions,
                        format!("HIP-3 clearinghouseState response parse failed: {e}"),
                    ),
                }
            }
            Ok(response) => completeness.mark_incomplete(
                AccountDataSection::Positions,
                format!(
                    "HIP-3 clearinghouseState request failed with HTTP {}",
                    response.status()
                ),
            ),
            Err(e) => completeness.mark_incomplete(
                AccountDataSection::Positions,
                format!("HIP-3 clearinghouseState request failed: {e}"),
            ),
        }
    }

    let mut hip3_order_sets = Vec::new();
    for (dex, resp) in hip3_dexes.iter().zip(hip3_ord_results) {
        match resp {
            Ok(response) if response.status().is_success() => {
                match response.json::<Vec<OpenOrder>>().await {
                    Ok(mut orders) => {
                        normalize_dex_open_order_coins(dex, &mut orders);
                        hip3_order_sets.push(orders);
                    }
                    Err(e) => completeness.mark_incomplete(
                        AccountDataSection::OpenOrders,
                        format!("HIP-3 frontendOpenOrders parse failed: {e}"),
                    ),
                }
            }
            Ok(response) => completeness.mark_incomplete(
                AccountDataSection::OpenOrders,
                format!(
                    "HIP-3 frontendOpenOrders request failed with HTTP {}",
                    response.status()
                ),
            ),
            Err(e) => completeness.mark_incomplete(
                AccountDataSection::OpenOrders,
                format!("HIP-3 frontendOpenOrders request failed: {e}"),
            ),
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
        fetched_at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
    })
}

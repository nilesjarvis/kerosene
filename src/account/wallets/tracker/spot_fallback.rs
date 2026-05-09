use super::super::super::{SpotClearinghouseState, spot::estimate_spot_equity};
use super::snapshot::parse_tracker_number;
use crate::api::API_URL;

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Portfolio-Margin Spot Fallback
// ---------------------------------------------------------------------------

pub(super) async fn apply_spot_equity_fallback(
    client: &reqwest::Client,
    address: &str,
    equity: &mut Option<f64>,
    withdrawable: &mut Option<f64>,
) -> Result<(), String> {
    if equity.is_some_and(|equity| equity > 0.0)
        && withdrawable.is_some_and(|withdrawable| withdrawable > 0.0)
    {
        return Ok(());
    }

    let spot_resp = client
        .post(API_URL)
        .json(&serde_json::json!({"type": "spotClearinghouseState", "user": address}))
        .send()
        .await
        .map_err(|e| format!("spotClearinghouseState request failed: {e}"))?;

    if !spot_resp.status().is_success() {
        return Err(format!(
            "spotClearinghouseState request failed with HTTP {}",
            spot_resp.status()
        ));
    }

    let spot_raw: serde_json::Value = spot_resp
        .json()
        .await
        .map_err(|e| format!("spotClearinghouseState parse failed: {e}"))?;

    let spot: SpotClearinghouseState = serde_json::from_value(spot_raw)
        .map_err(|e| format!("spotClearinghouseState deserialize failed: {e}"))?;

    if !spot.portfolio_margin_enabled {
        return Ok(());
    }

    let mids_resp = client
        .post(API_URL)
        .json(&serde_json::json!({"type": "allMids", "dex": ""}))
        .send()
        .await
        .map_err(|e| format!("allMids request failed: {e}"))?;

    if !mids_resp.status().is_success() {
        return Err(format!(
            "allMids request failed with HTTP {}",
            mids_resp.status()
        ));
    }

    let mids_raw: HashMap<String, String> = mids_resp
        .json()
        .await
        .map_err(|e| format!("allMids parse failed: {e}"))?;

    let mids: HashMap<String, f64> = mids_raw
        .into_iter()
        .filter_map(|(k, v)| {
            parse_tracker_number(&v)
                .filter(|price| *price > 0.0)
                .map(|price| (k, price))
        })
        .collect();

    if !equity.is_some_and(|equity| equity > 0.0) {
        *equity = estimate_spot_equity(&spot.balances, &mids);
    }

    if !withdrawable.is_some_and(|withdrawable| withdrawable > 0.0)
        && let Some(available_after_maintenance) = spot
            .token_to_available_after_maintenance
            .as_ref()
            .and_then(|v| v.iter().find(|(token, _)| *token == 0))
            .and_then(|(_, amount)| parse_tracker_number(amount))
    {
        *withdrawable = Some(available_after_maintenance);
    }

    Ok(())
}

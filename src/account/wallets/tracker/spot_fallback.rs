use super::super::super::{
    SpotClearinghouseState,
    spot::{augment_spot_balance_mids, estimate_spot_equity},
};
use super::snapshot::parse_tracker_number;
use crate::api::API_URL;

use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Portfolio-Margin Spot Fallback
// ---------------------------------------------------------------------------

/// Spot-derived equity values for a portfolio-margin wallet, whose perp
/// clearinghouse `accountValue`/`withdrawable` do not reflect the real
/// (spot-held) equity.
pub(super) struct SpotEquityFallback {
    /// Estimated USD value of the spot balances (`None` when a balance has
    /// no derivable value).
    pub(super) equity: Option<f64>,
    /// Token-0 (USDC) available balance after maintenance margin.
    pub(super) withdrawable: Option<f64>,
}

/// Derive fallback values from an already-fetched spot clearinghouse state.
/// Returns `None` for wallets without portfolio margin enabled.
pub(super) fn spot_equity_fallback_from_state(
    spot: &SpotClearinghouseState,
    mids: &HashMap<String, f64>,
) -> Option<SpotEquityFallback> {
    if !spot.portfolio_margin_enabled {
        return None;
    }
    let withdrawable = spot
        .token_to_available_after_maintenance
        .as_ref()
        .and_then(|tokens| tokens.iter().find(|(token, _)| *token == 0))
        .and_then(|(_, amount)| parse_tracker_number(amount));
    Some(SpotEquityFallback {
        equity: estimate_spot_equity(&spot.balances, mids),
        withdrawable,
    })
}

/// Merge a best-effort portfolio-margin spot outcome into the perp snapshot.
///
/// Errors (and non-portfolio-margin wallets) leave the already-fetched perp
/// values untouched: a transient failure of the auxiliary spot request must
/// never discard a valid perp snapshot.
pub(super) fn merge_spot_equity_fallback(
    outcome: Result<Option<SpotEquityFallback>, String>,
    equity: &mut Option<f64>,
    withdrawable: &mut Option<f64>,
) {
    let Ok(Some(fallback)) = outcome else {
        return;
    };
    // A returned fallback is known to be portfolio-margin spot state, which is
    // authoritative for these headline values. Do not let a small but
    // incomplete per-dex clearinghouse number override it.
    if fallback.equity.is_some() {
        *equity = fallback.equity;
    }
    if fallback.withdrawable.is_some() {
        *withdrawable = fallback.withdrawable;
    }
}

/// Best-effort spot-equity enrichment for the direct Hyperliquid path.
pub(super) async fn apply_spot_equity_fallback(
    client: &reqwest::Client,
    address: &str,
    equity: &mut Option<f64>,
    withdrawable: &mut Option<f64>,
) -> Option<String> {
    // Portfolio-margin clearinghouse values can be positive yet incomplete.
    // Always fetch spot state so PM detection does not depend on the magnitude
    // of those non-authoritative values.
    let outcome = fetch_spot_equity_fallback(client, address).await;
    let warning = match &outcome {
        Err(error) => Some(format!(
            "Spot/portfolio-margin valuation verification unavailable: {error}"
        )),
        Ok(Some(fallback)) if fallback.equity.is_none() => Some(
            "Portfolio-margin spot valuation is incomplete because one or more balances have no live mark"
                .to_string(),
        ),
        _ => None,
    };
    merge_spot_equity_fallback(outcome, equity, withdrawable);
    warning
}

async fn fetch_spot_equity_fallback(
    client: &reqwest::Client,
    address: &str,
) -> Result<Option<SpotEquityFallback>, String> {
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
        return Ok(None);
    }

    let mids = fetch_spot_fallback_mids(client).await?;
    Ok(spot_equity_fallback_from_state(&spot, &mids))
}

/// Fetch the all-mids map used to price portfolio-margin spot balances.
pub(super) async fn fetch_spot_fallback_mids(
    client: &reqwest::Client,
) -> Result<HashMap<String, f64>, String> {
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

    let mut mids: HashMap<String, f64> = mids_raw
        .into_iter()
        .filter_map(|(k, v)| {
            parse_tracker_number(&v)
                .filter(|price| *price > 0.0)
                .map(|price| (k, price))
        })
        .collect();
    if let Ok(Some(payload)) =
        crate::api_cache::load_fresh_exchange_symbols(crate::app_time::now_ms())
    {
        augment_spot_balance_mids(&mut mids, &payload.symbols);
    }
    Ok(mids)
}

use super::super::{ClearinghouseState, WalletTrackerSnapshot};
use crate::api::API_URL;

use serde_json::Value;

mod hip3;
mod open_orders;
mod snapshot;
mod spot_fallback;

use hip3::append_hip3_margin_and_positions;
pub use open_orders::fetch_wallet_tracker_open_order_count;
use snapshot::{build_wallet_tracker_snapshot, parse_tracker_number};
use spot_fallback::apply_spot_equity_fallback;

/// Fetch a low-cost account snapshot for the wallet tracker.
///
/// This intentionally excludes `openOrders`: those requests have much higher
/// rate-limit weight and are refreshed by a separate slow/manual lane.
pub async fn fetch_wallet_tracker_snapshot(
    address: String,
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
    append_hip3_margin_and_positions(&client, &address, &mut margin_used, &mut asset_positions)
        .await;

    Ok(build_wallet_tracker_snapshot(
        equity,
        withdrawable,
        margin_used,
        asset_positions,
    ))
}

use super::super::super::{AssetPosition, ClearinghouseState, HIP3_DEXES};
use super::snapshot::parse_tracker_number;
use crate::api::API_URL;

// ---------------------------------------------------------------------------
// HIP-3 Snapshot Aggregation
// ---------------------------------------------------------------------------

pub(super) async fn append_hip3_margin_and_positions(
    client: &reqwest::Client,
    address: &str,
    margin_used: &mut Option<f64>,
    asset_positions: &mut Vec<AssetPosition>,
) {
    let mut hip3_ch_futs = Vec::new();
    for dex in HIP3_DEXES {
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
    }

    for resp in futures::future::join_all(hip3_ch_futs).await {
        if let Ok(response) = resp
            && let Ok(raw) = response.json::<serde_json::Value>().await
            && let Ok(ch) = serde_json::from_value::<ClearinghouseState>(raw)
        {
            add_optional(
                margin_used,
                parse_tracker_number(&ch.margin_summary.total_margin_used),
            );
            asset_positions.extend(ch.asset_positions);
        }
    }
}

fn add_optional(total: &mut Option<f64>, value: Option<f64>) {
    *total = total.and_then(|total| value.map(|value| total + value));
}

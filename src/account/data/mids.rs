use crate::api::API_URL;
use crate::api::proxy::HyperliquidRequestExt;

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// All-Mids Fetches
// ---------------------------------------------------------------------------

/// Fetch all mid prices for a given dex.
pub async fn fetch_all_mids(dex: String) -> Result<HashMap<String, String>, String> {
    let client = &*crate::api::CLIENT;
    let mids: HashMap<String, String> = client
        .post(API_URL)
        .json(&serde_json::json!({"type": "allMids", "dex": dex}))
        .send_info()
        .await
        .map_err(|e| format!("allMids request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("allMids parse failed: {e}"))?;
    Ok(mids)
}

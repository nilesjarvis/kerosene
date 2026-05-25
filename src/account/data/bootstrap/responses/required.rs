use crate::account::{ClearinghouseState, SpotClearinghouseState};

use serde_json::Value;

// ---------------------------------------------------------------------------
// Required Bootstrap Responses
// ---------------------------------------------------------------------------

pub(in crate::account::data::bootstrap) fn clearinghouse_from_required_value(
    raw: Value,
) -> Result<ClearinghouseState, String> {
    serde_json::from_value(raw.clone()).map_err(|e| {
        format!(
            "clearinghouseState deserialize failed: {e} | JSON: {}",
            raw.to_string().chars().take(200).collect::<String>()
        )
    })
}

pub(in crate::account::data::bootstrap) fn spot_from_required_value(
    raw: Value,
) -> Result<SpotClearinghouseState, String> {
    serde_json::from_value(raw)
        .map_err(|e| format!("spotClearinghouseState deserialize failed: {e}"))
}

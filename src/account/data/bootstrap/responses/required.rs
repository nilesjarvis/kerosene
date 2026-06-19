use crate::account::{ClearinghouseState, SpotClearinghouseState};
use crate::helpers::sensitive_response_excerpt;

use serde_json::Value;

// ---------------------------------------------------------------------------
// Required Bootstrap Responses
// ---------------------------------------------------------------------------

const REQUIRED_RESPONSE_PREVIEW_CHARS: usize = 200;

pub(in crate::account::data::bootstrap) fn clearinghouse_from_required_value(
    raw: Value,
) -> Result<ClearinghouseState, String> {
    serde_json::from_value(raw.clone()).map_err(|e| {
        format!(
            "clearinghouseState deserialize failed: {e} | JSON: {}",
            required_response_preview(&raw.to_string())
        )
    })
}

pub(in crate::account::data::bootstrap) fn spot_from_required_value(
    raw: Value,
) -> Result<SpotClearinghouseState, String> {
    serde_json::from_value(raw)
        .map_err(|e| format!("spotClearinghouseState deserialize failed: {e}"))
}

fn required_response_preview(text: &str) -> String {
    sensitive_response_excerpt(text, REQUIRED_RESPONSE_PREVIEW_CHARS)
}

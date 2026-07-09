use crate::account::{
    AccountDataCompleteness, AccountDataSection, ClearinghouseState, MarginSummary,
    SpotClearinghouseState,
};
use crate::helpers::{redact_sensitive_response_text, sensitive_response_excerpt};

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

pub(in crate::account::data::bootstrap) fn account_states_from_required_spot(
    clearinghouse_raw: Result<Value, String>,
    spot_raw: Result<Value, String>,
) -> Result<
    (
        ClearinghouseState,
        SpotClearinghouseState,
        AccountDataCompleteness,
    ),
    String,
> {
    // Spot state is required for a connected account because spot percentage
    // sizing cannot safely infer balances. Main-perp state is independent: a
    // failure there must block perp position actions without discarding a
    // healthy spot snapshot.
    let spot = spot_from_required_value(spot_raw?)?;
    let mut completeness = AccountDataCompleteness::default();
    let clearinghouse = match clearinghouse_raw.and_then(clearinghouse_from_required_value) {
        Ok(clearinghouse) => clearinghouse,
        Err(error) => {
            completeness.mark_incomplete(
                AccountDataSection::Positions,
                format!(
                    "clearinghouseState unavailable: {}",
                    redact_sensitive_response_text(&error)
                ),
            );
            empty_clearinghouse_state()
        }
    };

    Ok((clearinghouse, spot, completeness))
}

fn empty_clearinghouse_state() -> ClearinghouseState {
    ClearinghouseState {
        margin_summary: MarginSummary {
            account_value: "0".to_string(),
            total_ntl_pos: "0".to_string(),
            total_margin_used: "0".to_string(),
        },
        cross_margin_summary: None,
        cross_maintenance_margin_used: None,
        withdrawable: "0".to_string(),
        asset_positions: Vec::new(),
    }
}

fn required_response_preview(text: &str) -> String {
    sensitive_response_excerpt(text, REQUIRED_RESPONSE_PREVIEW_CHARS)
}

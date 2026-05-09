use self::parsing::{parse_reserve_states, parse_spot_token_names};
use self::snapshot::build_income_snapshot;
use super::model::{BorrowLendInterestEntry, BorrowLendUserState, IncomeSnapshot};
use crate::api::{API_URL, CLIENT};

use serde_json::Value;
use std::collections::HashMap;

mod parsing;
mod snapshot;

// ---------------------------------------------------------------------------
// Fetching
// ---------------------------------------------------------------------------

/// Fetch borrow/lend income data for a portfolio-margin account.
pub async fn fetch_income_data(address: String) -> Result<IncomeSnapshot, String> {
    let client = CLIENT.clone();

    let reserve_fut = client
        .post(API_URL)
        .json(&serde_json::json!({"type": "allBorrowLendReserveStates"}))
        .send();
    let user_state_fut = client
        .post(API_URL)
        .json(&serde_json::json!({"type": "borrowLendUserState", "user": address}))
        .send();
    let interest_fut = client
        .post(API_URL)
        .json(&serde_json::json!({"type": "userBorrowLendInterest", "user": address}))
        .send();
    let spot_meta_fut = client
        .post(API_URL)
        .json(&serde_json::json!({"type": "spotMeta"}))
        .send();

    let (reserve_resp, user_state_resp, interest_resp, spot_meta_resp) =
        futures::future::join4(reserve_fut, user_state_fut, interest_fut, spot_meta_fut).await;

    let reserve_raw: Value = reserve_resp
        .map_err(|e| format!("allBorrowLendReserveStates request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("allBorrowLendReserveStates parse failed: {e}"))?;

    if let Some(err) = reserve_raw.get("error").and_then(|v| v.as_str()) {
        return Err(format!("allBorrowLendReserveStates error: {err}"));
    }

    let reserve_by_token = parse_reserve_states(&reserve_raw);
    if reserve_by_token.is_empty() {
        let preview = reserve_raw.to_string();
        let snippet = if preview.len() > 180 {
            format!("{}...", &preview[..180])
        } else {
            preview
        };
        return Err(format!(
            "allBorrowLendReserveStates response had no parseable reserve entries: {snippet}"
        ));
    }

    let user_state: BorrowLendUserState = user_state_resp
        .map_err(|e| format!("borrowLendUserState request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("borrowLendUserState parse failed: {e}"))?;

    let interest_entries: Vec<BorrowLendInterestEntry> = interest_resp
        .map_err(|e| format!("userBorrowLendInterest request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("userBorrowLendInterest parse failed: {e}"))?;

    let token_name_by_id: HashMap<u32, String> = match spot_meta_resp {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(raw) => parse_spot_token_names(&raw),
            Err(_) => HashMap::new(),
        },
        Err(_) => HashMap::new(),
    };

    Ok(build_income_snapshot(
        user_state,
        &interest_entries,
        &reserve_by_token,
        &token_name_by_id,
    ))
}

use crate::account_analytics::model::BorrowLendReserveState;
use serde_json::Value;
use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Reserve State Parsing
// ---------------------------------------------------------------------------

fn value_to_string(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        Some(text.to_string())
    } else if value.is_number() {
        Some(value.to_string())
    } else {
        None
    }
}

fn parse_reserve_entry(token: u32, value: &Value) -> Option<(u32, BorrowLendReserveState)> {
    let obj = value.as_object()?;
    Some((
        token,
        BorrowLendReserveState {
            borrow_yearly_rate: obj
                .get("borrowYearlyRate")
                .and_then(value_to_string)
                .unwrap_or_default(),
            supply_yearly_rate: obj
                .get("supplyYearlyRate")
                .and_then(value_to_string)
                .unwrap_or_default(),
            oracle_px: obj
                .get("oraclePx")
                .and_then(value_to_string)
                .unwrap_or_default(),
        },
    ))
}

pub(in crate::account_analytics::income) fn parse_reserve_states(
    raw: &Value,
) -> HashMap<u32, BorrowLendReserveState> {
    let mut out = HashMap::new();

    let parse_pair_array = |arr: &[Value], out: &mut HashMap<u32, BorrowLendReserveState>| {
        for entry in arr {
            let Some(pair) = entry.as_array() else {
                continue;
            };
            if pair.len() != 2 {
                continue;
            }
            let Some(token_u64) = pair[0].as_u64() else {
                continue;
            };
            let Ok(token) = u32::try_from(token_u64) else {
                continue;
            };
            if let Some((token, reserve)) = parse_reserve_entry(token, &pair[1]) {
                out.insert(token, reserve);
            }
        }
    };

    if let Some(arr) = raw.as_array() {
        parse_pair_array(arr, &mut out);
        return out;
    }

    let Some(obj) = raw.as_object() else {
        return out;
    };

    for key in ["reserveStates", "reserves", "data", "result"] {
        if let Some(arr) = obj.get(key).and_then(|value| value.as_array()) {
            parse_pair_array(arr, &mut out);
            if !out.is_empty() {
                return out;
            }
        }
    }

    for (key, value) in obj {
        let Ok(token) = key.parse::<u32>() else {
            continue;
        };
        if let Some((token, reserve)) = parse_reserve_entry(token, value) {
            out.insert(token, reserve);
        }
    }

    out
}

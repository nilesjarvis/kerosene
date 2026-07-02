use crate::account::UserFill;
use crate::helpers::{finite_value, non_perp_fee_usd, positive_finite_value};
use crate::signing::ExchangeResponse;

use std::collections::HashSet;

// ---------------------------------------------------------------------------
// TWAP Fill Summaries
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ResponseFillSummary {
    pub(crate) oid: Option<u64>,
    pub(crate) filled_size: f64,
    pub(crate) avg_price: Option<f64>,
}

pub(crate) fn twap_response_fill_summary(response: &ExchangeResponse) -> ResponseFillSummary {
    let mut summary = ResponseFillSummary::default();
    let Some(statuses) = response
        .response
        .as_ref()
        .and_then(|inner| inner.data.as_ref())
        .map(|data| data.statuses.as_slice())
    else {
        return summary;
    };

    for status in statuses {
        let Some(filled) = status.get("filled") else {
            continue;
        };
        if summary.oid.is_none() {
            summary.oid = filled.get("oid").and_then(|value| value.as_u64());
        }
        if let Some(size) = filled
            .get("totalSz")
            .and_then(|value| value.as_str())
            .and_then(|value| value.parse::<f64>().ok())
            .and_then(positive_finite_value)
        {
            summary.filled_size += size;
        }
        if summary.avg_price.is_none() {
            summary.avg_price = filled
                .get("avgPx")
                .and_then(|value| value.as_str())
                .and_then(|value| value.parse::<f64>().ok())
                .and_then(positive_finite_value);
        }
    }
    summary
}

#[derive(Debug, Clone, Copy)]
pub(super) struct FillSummary {
    pub(super) filled_size: f64,
    pub(super) avg_price: Option<f64>,
    pub(super) fee: f64,
}

pub(super) fn fill_summary_for_order(
    fills: &[UserFill],
    oid: u64,
    expected_coin: &str,
    expected_is_buy: bool,
) -> Option<FillSummary> {
    let mut filled_size = 0.0;
    let mut notional = 0.0;
    let mut fee = 0.0;
    let mut seen = HashSet::new();

    for fill in fills.iter().filter(|fill| fill.oid == Some(oid)) {
        if fill.coin != expected_coin || fill_side_is_buy(&fill.side) != Some(expected_is_buy) {
            continue;
        }
        if !seen.insert(fill.dedup_key()) {
            continue;
        }
        let Ok(size) = fill.sz.parse::<f64>() else {
            continue;
        };
        let Ok(price) = fill.px.parse::<f64>() else {
            continue;
        };
        let Some(size) = positive_finite_value(size) else {
            continue;
        };
        let Some(price) = positive_finite_value(price) else {
            continue;
        };
        filled_size += size;
        notional += size * price;
        if let Ok(parsed_fee) = fill.fee.parse::<f64>()
            && let Some(parsed_fee) = finite_value(parsed_fee)
        {
            // Spot buy fees arrive in the base token; convert at the fill
            // price so TWAP fee totals stay USD-denominated.
            let fee_token = fill.fee_token.as_deref().unwrap_or("");
            fee += non_perp_fee_usd(parsed_fee, fee_token, price).abs();
        }
    }

    if filled_size <= 0.0 {
        return None;
    }
    Some(FillSummary {
        filled_size,
        avg_price: Some(notional / filled_size),
        fee,
    })
}

fn fill_side_is_buy(side: &str) -> Option<bool> {
    match side {
        "B" => Some(true),
        "A" => Some(false),
        _ => None,
    }
}

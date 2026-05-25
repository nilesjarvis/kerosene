use crate::account::UserFill;
use crate::helpers::{finite_value, positive_finite_value};
use crate::signing::ExchangeResponse;

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

pub(super) fn fill_summary_for_oid(fills: &[UserFill], oid: u64) -> Option<FillSummary> {
    let mut filled_size = 0.0;
    let mut notional = 0.0;
    let mut fee = 0.0;

    for fill in fills.iter().filter(|fill| fill.oid == Some(oid)) {
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
            fee += parsed_fee.abs();
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

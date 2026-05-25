use crate::helpers::positive_finite_value;

use super::model::TwapOrder;

// ---------------------------------------------------------------------------
// TWAP Metrics
// ---------------------------------------------------------------------------

pub(crate) fn twap_weighted_average_fill_price(twap: &TwapOrder) -> Option<f64> {
    let mut size = 0.0;
    let mut notional = 0.0;
    for child in &twap.child_orders {
        let Some(filled_size) = positive_finite_value(child.filled_size) else {
            continue;
        };
        let Some(price) = child.avg_price.and_then(positive_finite_value) else {
            continue;
        };
        size += filled_size;
        notional += filled_size * price;
    }
    positive_finite_value(size).map(|size| notional / size)
}

#[cfg(test)]
mod tests;

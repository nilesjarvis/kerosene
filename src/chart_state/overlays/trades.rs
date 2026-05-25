use crate::account::UserFill;
use crate::chart::TradeMarker;
use crate::helpers::parse_positive_finite_number;

// ---------------------------------------------------------------------------
// Trade Marker Overlays
// ---------------------------------------------------------------------------

pub(super) fn trade_markers_for_symbol(fills: &[UserFill], symbol: &str) -> Vec<TradeMarker> {
    fills
        .iter()
        .filter(|fill| fill.coin == symbol)
        .filter_map(|fill| {
            let price = parse_positive_f64(&fill.px)?;
            let size = parse_positive_f64(&fill.sz)?;
            let is_buy = match fill.side.as_str() {
                "B" => true,
                "A" => false,
                _ => return None,
            };

            Some(TradeMarker {
                time_ms: fill.time,
                price,
                size,
                is_buy,
            })
        })
        .collect()
}

fn parse_positive_f64(raw: &str) -> Option<f64> {
    parse_positive_finite_number(raw)
}

#[cfg(test)]
mod tests;

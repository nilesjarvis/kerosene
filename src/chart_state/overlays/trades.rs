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
            let price = parse_positive_finite_number(&fill.px)?;
            let size = parse_positive_finite_number(&fill.sz)?;
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

#[cfg(test)]
mod tests;

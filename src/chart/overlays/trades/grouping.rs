use crate::api::Candle;
use crate::chart::model::TradeMarker;

use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Trade Marker Grouping
// ---------------------------------------------------------------------------

pub(super) const TRADE_MARKER_MAX_GROUPS: usize = 240;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TradeMarkerGroup {
    pub(super) candle_idx: usize,
    pub(super) is_buy: bool,
    pub(super) count: usize,
    pub(super) price: f64,
}

#[derive(Debug, Clone)]
struct TradeMarkerAccumulator {
    candle_idx: usize,
    is_buy: bool,
    count: usize,
    price_sum: f64,
    weighted_price_sum: f64,
    weight_sum: f64,
}

impl TradeMarkerAccumulator {
    fn new(candle_idx: usize, is_buy: bool) -> Self {
        Self {
            candle_idx,
            is_buy,
            count: 0,
            price_sum: 0.0,
            weighted_price_sum: 0.0,
            weight_sum: 0.0,
        }
    }

    fn add(&mut self, marker: &TradeMarker) {
        self.count += 1;
        self.price_sum += marker.price;
        if marker.size.is_finite() && marker.size > 0.0 {
            self.weighted_price_sum += marker.price * marker.size;
            self.weight_sum += marker.size;
        }
    }

    fn into_group(self) -> TradeMarkerGroup {
        let price = if self.weight_sum > 0.0 {
            self.weighted_price_sum / self.weight_sum
        } else if self.count > 0 {
            self.price_sum / self.count as f64
        } else {
            self.price_sum
        };

        TradeMarkerGroup {
            candle_idx: self.candle_idx,
            is_buy: self.is_buy,
            count: self.count,
            price,
        }
    }
}

pub(super) fn visible_trade_marker_groups(
    candles: &[Candle],
    markers: &[TradeMarker],
    first_vis: usize,
    last_vis: usize,
) -> Vec<TradeMarkerGroup> {
    if candles.is_empty() || markers.is_empty() || first_vis > last_vis || last_vis >= candles.len()
    {
        return Vec::new();
    }

    let visible_count = last_vis - first_vis + 1;
    let per_side_limit = (TRADE_MARKER_MAX_GROUPS / 2).max(1);
    let stride = visible_count.div_ceil(per_side_limit).max(1);
    let mut grouped: BTreeMap<(usize, bool), TradeMarkerAccumulator> = BTreeMap::new();

    let visible_start = candles[first_vis].open_time;
    let visible_end = candles[last_vis].close_time;
    let first_marker = markers.partition_point(|marker| marker.time_ms < visible_start);

    for marker in &markers[first_marker..] {
        if marker.time_ms > visible_end {
            break;
        }
        if !marker.price.is_finite() || marker.price <= 0.0 {
            continue;
        }

        let Some(candle_idx) = candle_index_for_time(candles, marker.time_ms) else {
            continue;
        };
        if candle_idx < first_vis || candle_idx > last_vis {
            continue;
        }

        let bucket_idx = bucket_candle_index(candle_idx, first_vis, last_vis, stride);
        grouped
            .entry((bucket_idx, marker.is_buy))
            .or_insert_with(|| TradeMarkerAccumulator::new(bucket_idx, marker.is_buy))
            .add(marker);
    }

    grouped
        .into_values()
        .filter(|group| group.count > 0)
        .map(TradeMarkerAccumulator::into_group)
        .collect()
}

fn candle_index_for_time(candles: &[Candle], time_ms: u64) -> Option<usize> {
    let idx = match candles.binary_search_by_key(&time_ms, |candle| candle.open_time) {
        Ok(idx) => idx,
        Err(0) => return None,
        Err(idx) => idx.saturating_sub(1),
    };

    candles.get(idx).and_then(|candle| {
        (time_ms >= candle.open_time && time_ms <= candle.close_time).then_some(idx)
    })
}

fn bucket_candle_index(
    candle_idx: usize,
    first_vis: usize,
    last_vis: usize,
    stride: usize,
) -> usize {
    if stride <= 1 {
        return candle_idx;
    }

    let bucket_start = first_vis + ((candle_idx - first_vis) / stride) * stride;
    bucket_start
        .saturating_add(stride / 2)
        .min(last_vis)
        .max(first_vis)
}

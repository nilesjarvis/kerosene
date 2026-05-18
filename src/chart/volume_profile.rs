use crate::api::{Candle, is_valid_candle};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Volume Profile
// ---------------------------------------------------------------------------

pub(in crate::chart) const VOLUME_PROFILE_MIN_BUCKETS: usize = 24;
pub(in crate::chart) const VOLUME_PROFILE_MAX_BUCKETS: usize = 120;
const VOLUME_PROFILE_TARGET_BUCKET_HEIGHT: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::chart) struct VolumeProfileBucket {
    pub(in crate::chart) price_lo: f64,
    pub(in crate::chart) price_hi: f64,
    pub(in crate::chart) volume: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::chart) struct VolumeProfile {
    pub(in crate::chart) buckets: Vec<VolumeProfileBucket>,
    pub(in crate::chart) max_volume: f64,
}

pub(in crate::chart) fn volume_profile_bucket_count(price_h: f32) -> usize {
    if !price_h.is_finite() || price_h <= 0.0 {
        return VOLUME_PROFILE_MIN_BUCKETS;
    }

    ((price_h / VOLUME_PROFILE_TARGET_BUCKET_HEIGHT).round() as usize)
        .clamp(VOLUME_PROFILE_MIN_BUCKETS, VOLUME_PROFILE_MAX_BUCKETS)
}

pub(in crate::chart) fn calculate_volume_profile(
    candles: &[Candle],
    price_lo: f64,
    price_hi: f64,
    bucket_count: usize,
) -> Option<VolumeProfile> {
    if candles.is_empty()
        || bucket_count == 0
        || !price_lo.is_finite()
        || !price_hi.is_finite()
        || price_hi <= price_lo
    {
        return None;
    }

    let bucket_height = (price_hi - price_lo) / bucket_count as f64;
    if bucket_height <= 0.0 || !bucket_height.is_finite() {
        return None;
    }

    let mut buckets: Vec<VolumeProfileBucket> = (0..bucket_count)
        .map(|idx| {
            let lo = price_lo + bucket_height * idx as f64;
            VolumeProfileBucket {
                price_lo: lo,
                price_hi: if idx + 1 == bucket_count {
                    price_hi
                } else {
                    lo + bucket_height
                },
                volume: 0.0,
            }
        })
        .collect();

    for candle in candles {
        accumulate_candle_volume(candle, price_lo, price_hi, bucket_height, &mut buckets);
    }

    let max_volume = buckets
        .iter()
        .map(|bucket| bucket.volume)
        .fold(0.0_f64, f64::max);
    if max_volume <= 0.0 {
        return None;
    }

    Some(VolumeProfile {
        buckets,
        max_volume,
    })
}

fn accumulate_candle_volume(
    candle: &Candle,
    price_lo: f64,
    price_hi: f64,
    bucket_height: f64,
    buckets: &mut [VolumeProfileBucket],
) {
    if !is_valid_candle(candle) || candle.volume <= 0.0 {
        return;
    }

    if candle.high == candle.low {
        if let Some(idx) = volume_profile_bucket_index(
            candle.low,
            price_lo,
            price_hi,
            bucket_height,
            buckets.len(),
        ) {
            buckets[idx].volume += candle.volume;
        }
        return;
    }

    if candle.high < price_lo || candle.low > price_hi {
        return;
    }

    let first_idx = volume_profile_bucket_index(
        candle.low.max(price_lo),
        price_lo,
        price_hi,
        bucket_height,
        buckets.len(),
    )
    .unwrap_or(0);
    let last_idx = volume_profile_bucket_index(
        candle.high.min(price_hi),
        price_lo,
        price_hi,
        bucket_height,
        buckets.len(),
    )
    .unwrap_or_else(|| buckets.len().saturating_sub(1));

    let candle_range = candle.high - candle.low;
    for idx in first_idx..=last_idx {
        let overlap_lo = candle.low.max(buckets[idx].price_lo).max(price_lo);
        let overlap_hi = candle.high.min(buckets[idx].price_hi).min(price_hi);
        let overlap = overlap_hi - overlap_lo;
        if overlap > 0.0 && overlap.is_finite() {
            buckets[idx].volume += candle.volume * overlap / candle_range;
        }
    }
}

fn volume_profile_bucket_index(
    price: f64,
    price_lo: f64,
    price_hi: f64,
    bucket_height: f64,
    bucket_count: usize,
) -> Option<usize> {
    if !price.is_finite()
        || !price_lo.is_finite()
        || !price_hi.is_finite()
        || !bucket_height.is_finite()
        || bucket_height <= 0.0
        || bucket_count == 0
        || price < price_lo
        || price > price_hi
    {
        return None;
    }

    let raw_idx = ((price - price_lo) / bucket_height).floor() as usize;
    Some(raw_idx.min(bucket_count.saturating_sub(1)))
}

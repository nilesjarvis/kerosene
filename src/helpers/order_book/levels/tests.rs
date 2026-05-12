use super::*;
use proptest::prelude::*;

fn lvl(px: f64, sz: f64) -> BookLevel {
    BookLevel { px, sz }
}

#[test]
fn empty_levels_aggregate_to_empty_buckets() {
    assert!(aggregate_levels(&[], 0.5, true).is_empty());
    assert!(aggregate_levels(&[], 0.5, false).is_empty());
}

#[test]
fn single_level_emits_one_bucket() {
    let asks = [lvl(100.5, 2.0)];
    let bids = [lvl(99.5, 2.0)];
    assert_eq!(aggregate_levels(&asks, 0.5, false), vec![(100.5, 2.0)]);
    assert_eq!(aggregate_levels(&bids, 0.5, true), vec![(99.5, 2.0)]);
}

/// Build a small set of book levels for proptest. Prices are positive and
/// well-spaced relative to a representative crypto mid; sizes are bounded
/// so the cumulative sums don't overflow f64 precision.
fn arb_levels() -> impl Strategy<Value = Vec<BookLevel>> {
    prop::collection::vec(
        (1.0f64..1_000_000.0f64, 0.0001f64..10_000.0f64).prop_map(|(px, sz)| lvl(px, sz)),
        0..32,
    )
}

proptest! {
    /// Bucketing must preserve total size: the sum across all buckets
    /// equals the sum across the input levels (modulo float rounding).
    #[test]
    fn bucketed_size_total_equals_input_size_total(
        levels in arb_levels(),
        tick_exp in -4i32..=2i32,
        is_bid in any::<bool>(),
    ) {
        let tick = 10f64.powi(tick_exp);
        let aggregated = aggregate_levels(&levels, tick, is_bid);
        let input_total: f64 = levels.iter().map(|l| l.sz).sum();
        let bucketed_total: f64 = aggregated.iter().map(|(_, sz)| sz).sum();
        let diff = (input_total - bucketed_total).abs();
        // Relative tolerance: f64 sums of ~32 levels lose a few ULPs.
        let tolerance = (input_total.abs() * 1e-9).max(1e-9);
        prop_assert!(
            diff <= tolerance,
            "mass leak: input {input_total}, bucketed {bucketed_total}, diff {diff}"
        );
    }

    /// Asks emerge from `aggregate_levels` in ascending price order;
    /// bids in descending price order. Best (inside) prices come first.
    #[test]
    fn aggregated_levels_are_sorted_inside_out(
        levels in arb_levels(),
        tick_exp in -4i32..=2i32,
    ) {
        let tick = 10f64.powi(tick_exp);
        let asks = aggregate_levels(&levels, tick, false);
        for window in asks.windows(2) {
            prop_assert!(
                window[0].0 <= window[1].0,
                "asks not ascending: {} then {}",
                window[0].0,
                window[1].0
            );
        }
        let bids = aggregate_levels(&levels, tick, true);
        for window in bids.windows(2) {
            prop_assert!(
                window[0].0 >= window[1].0,
                "bids not descending: {} then {}",
                window[0].0,
                window[1].0
            );
        }
    }

    /// Every bucket's price is an exact integer multiple of the tick — the
    /// `floor`/`ceil` keying must not produce off-grid prices.
    #[test]
    fn bucket_prices_are_exact_multiples_of_the_tick(
        levels in arb_levels(),
        tick_exp in -4i32..=2i32,
        is_bid in any::<bool>(),
    ) {
        let tick = 10f64.powi(tick_exp);
        let aggregated = aggregate_levels(&levels, tick, is_bid);
        for (price, _) in &aggregated {
            let key = (price / tick).round();
            let reconstructed = key * tick;
            let diff = (price - reconstructed).abs();
            // Allow a few ULPs at the bucket-price scale.
            let tolerance = (price.abs() * 1e-9).max(tick * 1e-9);
            prop_assert!(
                diff <= tolerance,
                "off-grid bucket: {price} not an integer multiple of {tick}"
            );
        }
    }

    /// For the ask side, every bucket price must be >= the highest input
    /// price that landed in it (ceil semantics). For the bid side it must
    /// be <= the lowest input price that landed in it (floor semantics).
    /// This catches accidental sign flips in the keying logic.
    #[test]
    fn ask_buckets_dominate_input_prices_via_ceil(
        levels in arb_levels(),
        tick_exp in -4i32..=2i32,
    ) {
        let tick = 10f64.powi(tick_exp);
        let aggregated = aggregate_levels(&levels, tick, false);
        for input in &levels {
            let bucket_price = (input.px / tick).ceil() * tick;
            let found = aggregated.iter().any(|(p, _)| {
                let diff = (p - bucket_price).abs();
                diff <= (bucket_price.abs() * 1e-9).max(tick * 1e-9)
            });
            prop_assert!(
                found,
                "ask input {} (bucket {}) missing from output",
                input.px,
                bucket_price
            );
        }
    }
}

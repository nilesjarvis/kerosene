use super::*;
use proptest::prelude::*;

mod deterministic;
mod properties;

fn lvl(px: f64, sz: f64) -> BookLevel {
    BookLevel { px, sz }
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

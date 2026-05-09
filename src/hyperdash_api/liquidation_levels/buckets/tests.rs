use crate::hyperdash_api::models::LiquidationEntry;

use super::bucket_liquidations;

#[test]
fn liquidation_buckets_split_long_and_short_notional() {
    let buckets = bucket_liquidations(
        &[
            LiquidationEntry {
                amount: 2.0,
                price: 105.0,
            },
            LiquidationEntry {
                amount: -1.5,
                price: 115.0,
            },
        ],
        100.0,
        120.0,
        2,
    );

    assert_eq!(buckets.len(), 2);
    assert_eq!(buckets[0].long_coins, 2.0);
    assert_eq!(buckets[0].long_usd, 210.0);
    assert_eq!(buckets[1].short_coins, 1.5);
    assert_eq!(buckets[1].short_usd, 172.5);
}

#[test]
fn liquidation_buckets_include_upper_bound_in_last_bucket() {
    let buckets = bucket_liquidations(
        &[LiquidationEntry {
            amount: -1.0,
            price: 120.0,
        }],
        100.0,
        120.0,
        2,
    );

    assert_eq!(buckets[1].short_coins, 1.0);
    assert_eq!(buckets[1].short_usd, 120.0);
}

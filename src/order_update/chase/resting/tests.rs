use super::chase_resting_reduce_only;
use crate::api::MarketType;

#[test]
fn resting_chase_preserves_known_perp_reduce_only_metadata() {
    assert_eq!(
        chase_resting_reduce_only(MarketType::Perp, Some(true)),
        Ok(true)
    );
    assert_eq!(
        chase_resting_reduce_only(MarketType::Perp, Some(false)),
        Ok(false)
    );
}

#[test]
fn resting_chase_rejects_unknown_perp_reduce_only_metadata() {
    assert!(
        chase_resting_reduce_only(MarketType::Perp, None)
            .expect_err("unknown reduce-only should be rejected")
            .contains("reduce-only metadata is unavailable")
    );
}

#[test]
fn resting_chase_ignores_spot_reduce_only_metadata() {
    assert_eq!(chase_resting_reduce_only(MarketType::Spot, None), Ok(false));
    assert_eq!(
        chase_resting_reduce_only(MarketType::Spot, Some(true)),
        Ok(false)
    );
}

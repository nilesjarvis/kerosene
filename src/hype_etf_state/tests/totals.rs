use super::*;

#[test]
fn totals_sum_values_and_weight_percentages_by_assets() {
    let data = HypeEtfData {
        funds: vec![
            fund(HypeEtfTicker::Thyp, 3_000_000.0, 1.0),
            fund(HypeEtfTicker::Bhyp, 1_000_000.0, 5.0),
        ],
        warnings: Vec::new(),
    };

    let totals = data.totals_for(HypeEtfView::All);

    assert_eq!(totals.net_assets_usd, Some(4_000_000.0));
    assert_eq!(totals.hype_exposure, Some(80_000.0));
    assert_eq!(totals.daily_volume, Some(2_000.0));
    assert_eq!(totals.weighted_premium_discount_pct, Some(2.0));
}

#[test]
fn totals_ignore_nonfinite_values_and_invalid_weights() {
    let mut invalid = fund(HypeEtfTicker::Bhyp, f64::NAN, 20.0);
    invalid.hype_exposure = Some(f64::INFINITY);
    invalid.shares_outstanding = Some(f64::NAN);
    invalid.daily_volume = Some(f64::INFINITY);
    let data = HypeEtfData {
        funds: vec![fund(HypeEtfTicker::Thyp, 1_000_000.0, 4.0), invalid],
        warnings: Vec::new(),
    };

    let totals = data.totals_for(HypeEtfView::All);

    assert_eq!(totals.net_assets_usd, Some(1_000_000.0));
    assert_eq!(totals.hype_exposure, Some(20_000.0));
    assert_eq!(totals.shares_outstanding, Some(10.0));
    assert_eq!(totals.daily_volume, Some(1_000.0));
    assert_eq!(totals.weighted_premium_discount_pct, Some(4.0));
}

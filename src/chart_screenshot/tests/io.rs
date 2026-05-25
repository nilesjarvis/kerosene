use super::*;

#[test]
fn chart_screenshot_filename_sanitizes_symbol_and_timeframe() {
    let at = local_time(2026, 5, 11, 15, 30);
    assert_eq!(
        chart_screenshot_filename("UBTC/USDC:PERP", "1H", at),
        "kerosene-UBTC-USDC-PERP-1H-20260511-153000.png"
    );
}

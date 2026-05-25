use super::*;

#[test]
fn usd_default_formats_like_existing_usd_formatter() {
    let ctx = DisplayDenominationContext::usd();

    assert_eq!(ctx.format_value(12_345.67, 2), "$12,345.67");
    assert_eq!(ctx.format_value(-12.5, 2), "-$12.50");
    assert_eq!(ctx.format_chart_price(125.0), "125.00");
}

#[test]
fn eur_context_converts_usd_by_usd_per_eur_mid() {
    let ctx = eur_context(1.25);

    assert_eq!(ctx.format_value(125.0, 2), "€100.00");
    assert_eq!(ctx.format_price(12.5), "10.00");
    assert_eq!(ctx.format_chart_price(125.0), "€100.00 ($125.00)");
}

#[test]
fn hype_context_uses_main_dex_mid_and_suffixes_unit() {
    let normalized_hype = DisplayDenominationConfig::Asset {
        code: " hype ".to_string(),
        dex: " ".to_string(),
        symbol: " hype ".to_string(),
    }
    .normalized();
    let ctx = hype_context(25.0);

    assert_eq!(normalized_hype, DisplayDenominationConfig::hype());
    assert_eq!(
        DisplayDenominationConfig::hype().rate_symbol_key(),
        Some("HYPE".to_string())
    );
    assert_eq!(ctx.active_symbol(), "HYPE");
    assert_eq!(ctx.format_value(125.0, 2), "5.00 HYPE");
    assert_eq!(ctx.format_signed_value(-125.0, 2), "-5.00 HYPE");
    assert_eq!(ctx.format_chart_price(125.0), "5.00 HYPE ($125.00)");
    assert_eq!(ctx.hidden_mask(), "*** HYPE");
}

#[test]
fn btc_context_uses_main_dex_mid_and_suffixes_unit() {
    let normalized_btc = DisplayDenominationConfig::Asset {
        code: " btc ".to_string(),
        dex: " ".to_string(),
        symbol: " btc ".to_string(),
    }
    .normalized();
    let ctx = btc_context(50_000.0);

    assert_eq!(normalized_btc, DisplayDenominationConfig::btc());
    assert_eq!(
        DisplayDenominationConfig::btc().rate_symbol_key(),
        Some("BTC".to_string())
    );
    assert_eq!(ctx.active_symbol(), "BTC");
    assert_eq!(ctx.format_value(100_000.0, 4), "2.0000 BTC");
    assert_eq!(ctx.format_signed_value(-100_000.0, 4), "-2.0000 BTC");
    assert_eq!(ctx.format_chart_price(100_000.0), "2.00 BTC ($100,000.0)");
    assert_eq!(ctx.hidden_mask(), "*** BTC");
}

#[test]
fn signed_values_keep_explicit_positive_marker() {
    let ctx = eur_context(2.0);

    assert_eq!(ctx.format_signed_value(20.0, 2), "+€10.00");
    assert_eq!(ctx.format_signed_value(-20.0, 2), "-€10.00");
    assert_eq!(ctx.format_signed_compact_value(2_000_000.0), "+€1.00M");
    assert_eq!(format_compact_usd(950.0), "$950");
    assert_eq!(format_compact_usd(1_500.0), "$1.5K");
    assert_eq!(format_compact_usd(-2_500_000.0), "-$2.50M");
}

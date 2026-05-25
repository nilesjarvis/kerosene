use super::*;

#[test]
fn symbol_match_checks_ticker_category_display_keywords_and_key() {
    let btc = symbol(
        "xyz:NVDA",
        "NVDA",
        "stocks",
        Some("Nvidia"),
        &["AI", "semiconductors"],
    );

    assert!(chart_editor_symbol_matches(&btc, ""));
    assert!(chart_editor_symbol_matches(&btc, "nvd"));
    assert!(chart_editor_symbol_matches(&btc, "stock"));
    assert!(chart_editor_symbol_matches(&btc, "nvidia"));
    assert!(chart_editor_symbol_matches(&btc, "semi"));
    assert!(chart_editor_symbol_matches(&btc, "xyz"));
    assert!(!chart_editor_symbol_matches(&btc, "btc"));
}

#[test]
fn symbol_match_hides_question_fallback_outcomes() {
    let fallback = fallback_outcome_symbol();

    assert!(!chart_editor_symbol_matches(&fallback, ""));
    assert!(!chart_editor_symbol_matches(&fallback, "fallback"));
    assert!(!chart_editor_symbol_matches(&fallback, "#660"));
}

#[test]
fn symbol_score_prioritizes_exact_and_prefix_matches() {
    let btc = symbol("BTC", "BTC", "crypto", Some("Bitcoin"), &["store of value"]);

    assert_eq!(chart_editor_symbol_score(&btc, ""), 0);
    assert_eq!(chart_editor_symbol_score(&btc, "btc"), 0);
    assert_eq!(chart_editor_symbol_score(&btc, "bit"), 1);
    assert_eq!(chart_editor_symbol_score(&btc, "coin"), 2);
}

use super::*;

#[test]
fn trade_draft_title_marks_buy_sell_direction() {
    let terminal = terminal_with_hype();
    let buy = trade_draft_or_panic(&terminal, "buy $1k HYPE");
    let sell = trade_draft_or_panic(&terminal, "sell $1k HYPE");

    assert_eq!(buy.title, "↑ BUY $1,000 HYPE");
    assert_eq!(buy.icon_title_anchor.as_deref(), Some("HYPE"));
    assert_eq!(sell.title, "↓ SELL $1,000 HYPE");
    assert_eq!(sell.icon_title_anchor.as_deref(), Some("HYPE"));
}

#[test]
fn bare_ticker_draft_prefers_perp_over_spot_market() {
    let terminal = terminal_with_hype_perp_and_spot();
    let draft = trade_draft_or_panic(&terminal, "sell 10 hype");

    assert_eq!(draft.symbol_key.as_deref(), Some("HYPE"));
    assert_eq!(draft.error, None);
}

#[test]
fn explicit_pair_spelling_resolves_to_spot_market() {
    let terminal = terminal_with_hype_perp_and_spot();
    let draft = trade_draft_or_panic(&terminal, "sell 10 hype/usdc");

    assert_eq!(draft.symbol_key.as_deref(), Some("@107"));
    assert_eq!(draft.error, None);
    assert_eq!(draft.title, "↓ SELL 10 HYPE/USDC");
    assert!(draft.can_submit());
}

#[test]
fn spot_qualifier_resolves_bare_ticker_to_spot_market() {
    let terminal = terminal_with_hype_perp_and_spot();
    let draft = trade_draft_or_panic(&terminal, "sell 10 hype spot");

    assert_eq!(draft.symbol_key.as_deref(), Some("@107"));
    assert_eq!(draft.error, None);
    assert_eq!(draft.title, "↓ SELL 10 HYPE/USDC");
}

#[test]
fn pair_spelling_never_falls_back_to_the_perp() {
    let terminal = terminal_with_hype();
    let draft = trade_draft_or_panic(&terminal, "sell 10 hype/usdc");

    assert_eq!(draft.symbol_key, None);
    assert_eq!(
        draft.error.as_deref(),
        Some("No spot market for 'HYPE/USDC'")
    );
    assert!(!draft.can_submit());
}

#[test]
fn spot_qualifier_without_spot_market_reports_missing_spot_market() {
    let terminal = terminal_with_hype();
    let draft = trade_draft_or_panic(&terminal, "sell 10 hype spot");

    assert_eq!(draft.symbol_key, None);
    assert_eq!(draft.error.as_deref(), Some("No spot market for 'HYPE'"));
    assert!(!draft.can_submit());
}

#[test]
fn chase_draft_without_side_can_be_applied() {
    let terminal = terminal_with_hype();
    let draft = trade_draft_or_panic(&terminal, "chase $1k HYPE");

    assert_eq!(draft.side, None);
    assert_eq!(draft.symbol_key.as_deref(), Some("HYPE"));
    assert_eq!(draft.order_kind, OrderKind::Chase);
    assert_eq!(draft.title, "CHASE $1,000 HYPE");
    assert_eq!(draft.detail, "Chase order, USD notional");
    assert_eq!(draft.tag, "Chase");
    assert!(draft.can_submit());
}

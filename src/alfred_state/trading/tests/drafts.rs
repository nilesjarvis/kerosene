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

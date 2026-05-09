use crate::feed_state::TrackedTradeIntent;

pub(super) fn tracked_trade_side_label(is_buy: bool) -> &'static str {
    if is_buy { "BUY" } else { "SELL" }
}

pub(super) fn tracked_trade_fee_label(fee: f64, fee_token: &str) -> String {
    if fee_token.trim().is_empty() {
        format!("{fee:.4}")
    } else {
        format!("{fee:.4} {fee_token}")
    }
}

pub(super) fn tracked_trade_intent_text(
    intent: TrackedTradeIntent,
    dir: &str,
    fill_count: usize,
) -> String {
    let intent_text = if intent == TrackedTradeIntent::Unknown && !dir.is_empty() {
        dir.to_string()
    } else {
        intent.label().to_string()
    };

    if fill_count > 1 {
        format!("{intent_text} x{fill_count}")
    } else {
        intent_text
    }
}

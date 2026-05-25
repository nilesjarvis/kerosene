use crate::twap_state::TwapPauseReason;

// ---------------------------------------------------------------------------
// TWAP Exchange Error Classification
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::order_execution::twap) enum TwapExchangeErrorAction {
    Retry(TwapPauseReason),
    Terminal,
    ConsumeSlice,
}

pub(in crate::order_execution::twap) fn classify_twap_exchange_error(
    summary: &str,
) -> TwapExchangeErrorAction {
    let summary = summary.to_ascii_lowercase();
    if summary.contains("rate limit")
        || summary.contains("ratelimit")
        || summary.contains("too many requests")
        || summary.contains("429")
        || summary.contains("temporarily")
        || summary.contains("unavailable")
        || summary.contains("overloaded")
        || summary.contains("try again")
    {
        return TwapExchangeErrorAction::Retry(TwapPauseReason::RateLimited);
    }

    if summary.contains("signature")
        || summary.contains("agent")
        || summary.contains("unauthorized")
        || summary.contains("not approved")
        || summary.contains("minimum")
        || summary.contains("min trade")
        || summary.contains("notional")
        || summary.contains("tick")
        || summary.contains("insufficient")
        || summary.contains("margin")
        || summary.contains("balance")
        || summary.contains("reduce only")
        || summary.contains("reduce-only")
        || summary.contains("open interest")
        || summary.contains("oracle")
        || summary.contains("delist")
        || summary.contains("max position")
    {
        return TwapExchangeErrorAction::Terminal;
    }

    TwapExchangeErrorAction::ConsumeSlice
}

pub(in crate::order_execution::twap) fn twap_terminal_cancel_error(summary: &str) -> bool {
    let summary = summary.to_ascii_lowercase();
    summary.contains("filled")
        || summary.contains("canceled")
        || summary.contains("cancelled")
        || summary.contains("cancled")
        || summary.contains("never placed")
        || summary.contains("not found")
        || summary.contains("does not exist")
        || summary.contains("no open order")
        || summary.contains("no longer open")
}

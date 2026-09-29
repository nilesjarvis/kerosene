// ---------------------------------------------------------------------------
// Exchange Error Text
// ---------------------------------------------------------------------------

/// Recognizes possibly settled cancellations; callers decide how to reconcile.
pub(crate) fn cancel_error_indicates_closed_order(summary: &str) -> bool {
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

/// Recognizes retryable exchange failures without assigning a recovery policy.
pub(crate) fn retryable_exchange_error(summary: &str) -> bool {
    retryable_exchange_error_lowercase(&summary.to_ascii_lowercase())
}

/// Reuses an ASCII-lowercased summary when the caller has more text to classify.
pub(super) fn retryable_exchange_error_lowercase(summary: &str) -> bool {
    summary.contains("rate limit")
        || summary.contains("ratelimit")
        || summary.contains("too many requests")
        || summary.contains("429")
        || summary.contains("temporarily")
        || summary.contains("unavailable")
        || summary.contains("overloaded")
        || summary.contains("try again")
}

#[cfg(test)]
mod tests;

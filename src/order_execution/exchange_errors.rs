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

#[cfg(test)]
mod tests;

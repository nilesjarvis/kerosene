// ---------------------------------------------------------------------------
// Chart Formatting
// ---------------------------------------------------------------------------

/// Format a dollar amount compactly: 1.23M, 345.1K, 1234, etc.
pub(super) fn format_compact(val: f64) -> String {
    if val >= 1_000_000.0 {
        format!("{:.2}M", val / 1_000_000.0)
    } else if val >= 1_000.0 {
        format!("{:.1}K", val / 1_000.0)
    } else {
        format!("{:.0}", val)
    }
}

/// Format a coin amount compactly, with appropriate decimal places.
pub(super) fn format_compact_coins(val: f64) -> String {
    if val >= 1_000_000.0 {
        format!("{:.2}M", val / 1_000_000.0)
    } else if val >= 1_000.0 {
        format!("{:.1}K", val / 1_000.0)
    } else if val >= 1.0 {
        format!("{:.2}", val)
    } else {
        format!("{:.4}", val)
    }
}

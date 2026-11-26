use crate::app_time::DisplayTimeZone;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Account History Formatting
// ---------------------------------------------------------------------------

pub(super) fn format_history_time_millis(ms: u64, zone: DisplayTimeZone) -> String {
    zone.format(ms, "%m/%d %H:%M", "--/-- --:--")
}

use crate::market_sessions::MarketSession;
#[cfg(test)]
use crate::market_sessions::{market_clock_text_for_window, market_window_is_active};
use chrono::{DateTime, Utc};
#[cfg(test)]
use chrono_tz::Tz;

// ---------------------------------------------------------------------------
// Market Session Clock Adapters
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(super) fn market_is_active(
    now_utc: DateTime<Utc>,
    tz: Tz,
    open: (u32, u32),
    close: (u32, u32),
) -> bool {
    market_window_is_active(now_utc, tz, open, close)
}

pub(super) fn session_is_active(now_utc: DateTime<Utc>, session: MarketSession) -> bool {
    session.market_is_active(now_utc)
}

#[cfg(test)]
pub(super) fn market_clock_text(
    label: &str,
    now_utc: DateTime<Utc>,
    tz: Tz,
    open: (u32, u32),
    close: (u32, u32),
) -> String {
    market_clock_text_for_window(label, now_utc, tz, open, close)
}

pub(super) fn session_clock_text(now_utc: DateTime<Utc>, session: MarketSession) -> Option<String> {
    session.market_clock_text(now_utc)
}

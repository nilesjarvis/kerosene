use crate::market_sessions::{MarketSession, SessionAnchor};

// ---------------------------------------------------------------------------
// Session-based starting points
// ---------------------------------------------------------------------------

/// Trading session for the normalization base time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Session {
    /// Most recent U.S. market open (9:30 AM ET).
    US,
    /// Most recent London market open (8:00 AM local time).
    Europe,
    /// Most recent Asian market open (9:00 AM JST = 0:00 UTC).
    Asia,
    /// Most recent UTC midnight.
    UtcDay,
    /// Most recent UTC week start (Monday 00:00 UTC).
    UtcWeek,
    /// Most recent UTC month start (day 1, 00:00 UTC).
    UtcMonth,
    /// Most recent UTC year start (Jan 1, 00:00 UTC).
    UtcYear,
}

impl Session {
    pub fn label(self) -> &'static str {
        match self {
            Session::US => "US",
            Session::Europe => "EU",
            Session::Asia => "Asia",
            Session::UtcDay => "UTC D",
            Session::UtcWeek => "UTC W",
            Session::UtcMonth => "UTC M",
            Session::UtcYear => "UTC Y",
        }
    }

    pub fn config_str(self) -> &'static str {
        match self {
            Session::US => "us",
            Session::Europe => "eu",
            Session::Asia => "asia",
            Session::UtcDay => "utc_day",
            Session::UtcWeek => "utc_week",
            Session::UtcMonth => "utc_month",
            Session::UtcYear => "utc_year",
        }
    }

    pub fn from_config_str(s: &str) -> Option<Self> {
        match s {
            "us" => Some(Session::US),
            "eu" => Some(Session::Europe),
            "asia" => Some(Session::Asia),
            "utc_day" => Some(Session::UtcDay),
            "utc_week" => Some(Session::UtcWeek),
            "utc_month" => Some(Session::UtcMonth),
            "utc_year" => Some(Session::UtcYear),
            _ => None,
        }
    }

    fn anchor(self) -> SessionAnchor {
        match self {
            Session::US => SessionAnchor::Market(MarketSession::NewYork),
            Session::Europe => SessionAnchor::Market(MarketSession::London),
            Session::Asia => SessionAnchor::Market(MarketSession::Asia),
            Session::UtcDay => SessionAnchor::UtcDay,
            Session::UtcWeek => SessionAnchor::UtcWeek,
            Session::UtcMonth => SessionAnchor::UtcMonth,
            Session::UtcYear => SessionAnchor::UtcYear,
        }
    }

    /// Compute the most recent session open timestamp (in ms) before
    /// `reference_ms`, using timezone-aware equity session opens.
    pub fn last_open_ms(self, reference_ms: u64) -> u64 {
        self.anchor().last_open_ms(reference_ms)
    }
}

pub const SESSION_OPTIONS: &[Session] = &[
    Session::US,
    Session::Europe,
    Session::Asia,
    Session::UtcDay,
    Session::UtcWeek,
    Session::UtcMonth,
    Session::UtcYear,
];

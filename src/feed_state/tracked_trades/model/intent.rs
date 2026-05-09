// ---------------------------------------------------------------------------
// Tracked Trade Intent
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrackedTradeIntent {
    Opening,
    Increasing,
    Reducing,
    Closing,
    Reversing,
    Unknown,
}

impl TrackedTradeIntent {
    pub(crate) fn from_positions(start_position: Option<f64>, signed_size_delta: f64) -> Self {
        let Some(start) = start_position else {
            return Self::Unknown;
        };

        let end = start + signed_size_delta;
        let eps = 1e-9;
        if start.abs() <= eps && end.abs() > eps {
            Self::Opening
        } else if end.abs() <= eps {
            Self::Closing
        } else if start.signum() != end.signum() {
            Self::Reversing
        } else if end.abs() > start.abs() {
            Self::Increasing
        } else if end.abs() < start.abs() {
            Self::Reducing
        } else {
            Self::Unknown
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Opening => "Opening",
            Self::Increasing => "Increasing",
            Self::Reducing => "Reducing",
            Self::Closing => "Closing",
            Self::Reversing => "Reversing",
            Self::Unknown => "Unknown",
        }
    }
}

use super::SessionDataId;
use crate::api::Candle;
use crate::market_sessions::MarketSession;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) enum SessionDataLookback {
    #[default]
    FourWeeks,
    EightWeeks,
    ThreeMonths,
    SixMonths,
    OneYear,
}

impl SessionDataLookback {
    pub(crate) const ALL: [Self; 5] = [
        Self::FourWeeks,
        Self::EightWeeks,
        Self::ThreeMonths,
        Self::SixMonths,
        Self::OneYear,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::FourWeeks => "4W",
            Self::EightWeeks => "8W",
            Self::ThreeMonths => "3M",
            Self::SixMonths => "6M",
            Self::OneYear => "1Y",
        }
    }

    pub(crate) fn days(self) -> u64 {
        match self {
            Self::FourWeeks => 28,
            Self::EightWeeks => 56,
            Self::ThreeMonths => 90,
            Self::SixMonths => 180,
            Self::OneYear => 365,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionWeekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl SessionWeekday {
    pub(crate) const ALL: [Self; 7] = [
        Self::Mon,
        Self::Tue,
        Self::Wed,
        Self::Thu,
        Self::Fri,
        Self::Sat,
        Self::Sun,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Mon => "Mon",
            Self::Tue => "Tue",
            Self::Wed => "Wed",
            Self::Thu => "Thu",
            Self::Fri => "Fri",
            Self::Sat => "Sat",
            Self::Sun => "Sun",
        }
    }

    pub(crate) fn index(self) -> usize {
        match self {
            Self::Mon => 0,
            Self::Tue => 1,
            Self::Wed => 2,
            Self::Thu => 3,
            Self::Fri => 4,
            Self::Sat => 5,
            Self::Sun => 6,
        }
    }

    pub(super) fn from_chrono(value: chrono::Weekday) -> Self {
        match value {
            chrono::Weekday::Mon => Self::Mon,
            chrono::Weekday::Tue => Self::Tue,
            chrono::Weekday::Wed => Self::Wed,
            chrono::Weekday::Thu => Self::Thu,
            chrono::Weekday::Fri => Self::Fri,
            chrono::Weekday::Sat => Self::Sat,
            chrono::Weekday::Sun => Self::Sun,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SessionReturnBar {
    pub(crate) open_time: u64,
    pub(crate) close_time: u64,
    pub(crate) weekday: SessionWeekday,
    pub(crate) open: f64,
    pub(crate) close: f64,
    pub(crate) volume: f64,
    pub(crate) return_pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SessionWeekdaySummary {
    pub(crate) weekday: SessionWeekday,
    pub(crate) sample_count: usize,
    pub(crate) average_return_pct: f64,
    pub(crate) win_rate_pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MarketSessionReturnBar {
    pub(crate) kind: MarketSession,
    pub(crate) start_ms: u64,
    pub(crate) end_ms: u64,
    pub(crate) open: f64,
    pub(crate) close: f64,
    pub(crate) return_pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MarketSessionSummary {
    pub(crate) session: MarketSession,
    pub(crate) sample_count: usize,
    pub(crate) average_return_pct: f64,
    pub(crate) win_rate_pct: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct SessionDataCandles {
    pub(crate) daily: Vec<Candle>,
    pub(crate) intraday: Vec<Candle>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionDataRequest {
    pub(crate) id: SessionDataId,
    pub(crate) symbol: String,
    pub(crate) lookback: SessionDataLookback,
    pub(crate) requested_at_ms: u64,
}

impl SessionDataRequest {
    pub(crate) fn matches_refresh_target(
        &self,
        id: SessionDataId,
        symbol: &str,
        lookback: SessionDataLookback,
    ) -> bool {
        self.id == id && self.symbol == symbol && self.lookback == lookback
    }
}

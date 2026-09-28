mod instance;
mod model;
mod returns;
mod statistics;

pub(crate) use instance::SessionDataInstance;
pub(crate) use model::{SessionDataCandles, SessionDataLookback, SessionDataRequest};
pub(crate) use statistics::{
    SessionGroup, SessionStreak, SessionVerdict, average_abs_move_pct, current_streak,
    most_active_weekday, overall_win_rate_pct, session_verdict, total_return_pct,
    weekday_dispersions,
};

pub(crate) type SessionDataId = u64;

#[cfg(test)]
mod tests;

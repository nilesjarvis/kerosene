use crate::app_state::TradingTerminal;
use crate::app_time::{local_datetime_from_unix_ms, utc_datetime_from_unix_ms};
use crate::helpers;
use crate::market_sessions::{MARKET_CLOCK_SESSIONS, MarketSession};
use crate::message::Message;
use chrono::{DateTime, Local, Timelike, Utc};
use iced::widget::{Row, row, text};

use super::connectivity::widgets::status_element_tooltip;

mod session;

#[cfg(test)]
use session::{market_clock_text, market_is_active};
use session::{session_clock_text, session_is_active};

// ---------------------------------------------------------------------------
// Status Clock Row
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn status_clock_row(&self, separated: bool) -> Row<'static, Message> {
        let theme = self.theme();
        let (now_utc, local_now) = status_clock_times(self.status_bar_now_ms);
        let local_text = format!(
            "Local {:02}:{:02}:{:02} {}",
            local_now.hour(),
            local_now.minute(),
            local_now.second(),
            local_now.format("%Z")
        );

        let row = row![text(local_text).size(10).color(theme.palette().primary)]
            .spacing(8)
            .align_y(iced::Alignment::Center);

        push_clock_gap(row, separated).push(status_element_tooltip(
            text(market_session_summary(now_utc))
                .size(10)
                .color(theme.palette().primary),
            market_session_tooltip(now_utc),
        ))
    }
}

fn status_clock_times(now_ms: u64) -> (DateTime<Utc>, DateTime<Local>) {
    (
        utc_datetime_from_unix_ms(now_ms),
        local_datetime_from_unix_ms(now_ms),
    )
}

fn push_clock_gap(row: Row<'static, Message>, separated: bool) -> Row<'static, Message> {
    if separated {
        row.push(helpers::vertical_spacer())
    } else {
        row
    }
}

fn market_session_summary(now_utc: DateTime<Utc>) -> String {
    let active: Vec<_> = MARKET_CLOCK_SESSIONS
        .into_iter()
        .filter(|session| session_is_active(now_utc, *session))
        .map(MarketSession::label)
        .collect();

    if active.is_empty() {
        "Markets closed".to_string()
    } else {
        format!("{} open", active.join(" + "))
    }
}

fn market_session_tooltip(now_utc: DateTime<Utc>) -> String {
    let sessions = MARKET_CLOCK_SESSIONS
        .into_iter()
        .map(|session| {
            let state = if session_is_active(now_utc, session) {
                "OPEN"
            } else {
                "CLOSED"
            };
            let detail =
                session_clock_text(now_utc, session).unwrap_or_else(|| session.label().to_string());
            format!("{state}  {detail}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("Market sessions\n\n{sessions}")
}

#[cfg(test)]
mod tests;

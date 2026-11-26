use crate::app_state::TradingTerminal;
use crate::app_time::{local_datetime_from_unix_ms, utc_datetime_from_unix_ms};
use crate::helpers;
use crate::market_sessions::{MARKET_CLOCK_SESSIONS, MarketClock, MarketClockState};
use crate::message::Message;
use chrono::{DateTime, Local, Timelike, Utc};
use iced::widget::{Row, row, text};

use super::connectivity::widgets::status_element_tooltip;

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

        let clocks = MARKET_CLOCK_SESSIONS.map(|session| session.clock_at(now_utc));
        push_clock_gap(row, separated).push(status_element_tooltip(
            text(market_session_summary(&clocks))
                .size(10)
                .color(theme.palette().primary),
            market_session_tooltip(&clocks),
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

fn market_session_summary(clocks: &[MarketClock]) -> String {
    let active: Vec<_> = clocks
        .iter()
        .filter(|clock| matches!(clock.state, MarketClockState::Open { .. }))
        .map(|clock| clock.session.label())
        .collect();

    if !active.is_empty() {
        format!("{} open", active.join(" + "))
    } else if clocks
        .iter()
        .any(|clock| matches!(clock.state, MarketClockState::Unavailable))
    {
        "Market hours unavailable".to_string()
    } else {
        "Markets closed".to_string()
    }
}

fn market_session_tooltip(clocks: &[MarketClock]) -> String {
    let sessions = clocks
        .iter()
        .map(MarketClock::tooltip_line)
        .collect::<Vec<_>>()
        .join("\n");

    format!("Market sessions\n\n{sessions}")
}

#[cfg(test)]
mod tests;

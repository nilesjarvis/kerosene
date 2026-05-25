use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use chrono::{DateTime, Local, Timelike, Utc};
use chrono_tz::Tz;
use iced::widget::container as container_style;
use iced::widget::{Row, Space, container, row, text};
use iced::{Color, Element, Theme};

mod session;

use session::{market_clock_text, market_is_active};

// ---------------------------------------------------------------------------
// Status Clock Row
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn status_clock_row(&self, separated: bool) -> Row<'static, Message> {
        let theme = self.theme();
        let now_utc = Utc::now();
        let local_now = Local::now();
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

        let row = push_clock_gap(row, separated).push(market_clock(
            "Europe",
            now_utc,
            chrono_tz::Europe::London,
            (8, 0),
            (16, 30),
            &theme,
        ));
        let row = push_clock_gap(row, separated).push(market_clock(
            "Asia",
            now_utc,
            chrono_tz::Asia::Tokyo,
            (9, 0),
            (15, 0),
            &theme,
        ));

        push_clock_gap(row, separated).push(market_clock(
            "New York",
            now_utc,
            chrono_tz::America::New_York,
            (9, 30),
            (16, 0),
            &theme,
        ))
    }
}

fn push_clock_gap(row: Row<'static, Message>, separated: bool) -> Row<'static, Message> {
    if separated {
        row.push(helpers::vertical_spacer())
    } else {
        row
    }
}

fn market_clock(
    label: &str,
    now_utc: DateTime<Utc>,
    tz: Tz,
    open: (u32, u32),
    close: (u32, u32),
    theme: &Theme,
) -> Element<'static, Message> {
    let is_active = market_is_active(now_utc, tz, open, close);
    let dot_color = if is_active {
        theme.palette().success
    } else {
        theme.palette().danger
    };
    let clock_text = market_clock_text(label, now_utc, tz, open, close);

    row![
        market_activity_dot(dot_color),
        text(clock_text).size(10).color(theme.palette().primary),
    ]
    .spacing(5)
    .align_y(iced::Alignment::Center)
    .into()
}

fn market_activity_dot(color: Color) -> Element<'static, Message> {
    container(Space::new().width(7).height(7))
        .style(move |_theme: &Theme| container_style::Style {
            background: Some(color.into()),
            border: iced::Border {
                radius: 10.0.into(),
                width: 1.0,
                color: Color { a: 0.32, ..color },
            },
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
mod tests;

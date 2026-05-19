use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use chrono::{DateTime, Datelike, Duration, Local, TimeZone, Timelike, Utc, Weekday};
use chrono_tz::Tz;
use iced::widget::container as container_style;
use iced::widget::{Row, Space, container, row, text};
use iced::{Color, Element, Theme};

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

fn format_market_time_until(now_utc: DateTime<Utc>, target_utc: DateTime<Utc>) -> String {
    let diff = target_utc - now_utc;
    let total_minutes = diff.num_minutes().max(0);
    let days = total_minutes / (24 * 60);
    let hours = (total_minutes % (24 * 60)) / 60;
    let minutes = total_minutes % 60;

    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else {
        format!("{hours}h {minutes}m")
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

fn next_market_open(
    now_utc: DateTime<Utc>,
    tz: Tz,
    hour: u32,
    minute: u32,
) -> Option<DateTime<Tz>> {
    let local_now = now_utc.with_timezone(&tz);
    for day_offset in 0..8 {
        let date = local_now.date_naive() + Duration::days(day_offset);
        if matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
            continue;
        }
        let naive = date.and_hms_opt(hour, minute, 0)?;
        let open_local = tz
            .from_local_datetime(&naive)
            .earliest()
            .or_else(|| tz.from_local_datetime(&naive).latest())?;
        if open_local > local_now {
            return Some(open_local);
        }
    }
    None
}

fn current_market_close(
    now_utc: DateTime<Utc>,
    tz: Tz,
    hour: u32,
    minute: u32,
) -> Option<DateTime<Tz>> {
    let local_now = now_utc.with_timezone(&tz);
    let close_local = local_time_for_date(tz, local_now.date_naive(), hour, minute)?;
    (close_local > local_now).then_some(close_local)
}

fn market_is_active(now_utc: DateTime<Utc>, tz: Tz, open: (u32, u32), close: (u32, u32)) -> bool {
    let local_now = now_utc.with_timezone(&tz);
    let date = local_now.date_naive();
    if matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
        return false;
    }

    let Some(open_local) = local_time_for_date(tz, date, open.0, open.1) else {
        return false;
    };
    let Some(close_local) = local_time_for_date(tz, date, close.0, close.1) else {
        return false;
    };

    local_now >= open_local && local_now < close_local
}

fn local_time_for_date(
    tz: Tz,
    date: chrono::NaiveDate,
    hour: u32,
    minute: u32,
) -> Option<DateTime<Tz>> {
    let naive = date.and_hms_opt(hour, minute, 0)?;
    tz.from_local_datetime(&naive)
        .earliest()
        .or_else(|| tz.from_local_datetime(&naive).latest())
}

fn market_clock_text(
    label: &str,
    now_utc: DateTime<Utc>,
    tz: Tz,
    open: (u32, u32),
    close: (u32, u32),
) -> String {
    let now_local = now_utc.with_timezone(&tz);
    let session_status = if market_is_active(now_utc, tz, open, close) {
        let close_in = current_market_close(now_utc, tz, close.0, close.1)
            .map(|dt| format_market_time_until(now_utc, dt.with_timezone(&Utc)))
            .unwrap_or_else(|| "n/a".to_string());
        format!("closes in {close_in}")
    } else {
        let next_open = next_market_open(now_utc, tz, open.0, open.1)
            .map(|dt| format_market_time_until(now_utc, dt.with_timezone(&Utc)))
            .unwrap_or_else(|| "n/a".to_string());
        format!("opens in {next_open}")
    };
    format!(
        "{label} {:02}:{:02}:{:02} {} ({session_status})",
        now_local.hour(),
        now_local.minute(),
        now_local.second(),
        now_local.format("%Z")
    )
}

#[cfg(test)]
mod tests {
    use super::{market_clock_text, market_is_active};
    use chrono::{TimeZone, Utc};

    #[test]
    fn market_activity_uses_local_session_hours() {
        assert!(market_is_active(
            Utc.with_ymd_and_hms(2026, 5, 18, 14, 0, 0)
                .single()
                .expect("valid UTC timestamp"),
            chrono_tz::America::New_York,
            (9, 30),
            (16, 0),
        ));
        assert!(!market_is_active(
            Utc.with_ymd_and_hms(2026, 5, 18, 13, 0, 0)
                .single()
                .expect("valid UTC timestamp"),
            chrono_tz::America::New_York,
            (9, 30),
            (16, 0),
        ));
        assert!(!market_is_active(
            Utc.with_ymd_and_hms(2026, 5, 18, 20, 30, 0)
                .single()
                .expect("valid UTC timestamp"),
            chrono_tz::America::New_York,
            (9, 30),
            (16, 0),
        ));
    }

    #[test]
    fn market_activity_respects_weekends() {
        assert!(!market_is_active(
            Utc.with_ymd_and_hms(2026, 5, 16, 14, 0, 0)
                .single()
                .expect("valid UTC timestamp"),
            chrono_tz::America::New_York,
            (9, 30),
            (16, 0),
        ));
    }

    #[test]
    fn market_activity_handles_regional_timezones() {
        assert!(market_is_active(
            Utc.with_ymd_and_hms(2026, 5, 18, 9, 0, 0)
                .single()
                .expect("valid UTC timestamp"),
            chrono_tz::Europe::London,
            (8, 0),
            (16, 30),
        ));
        assert!(market_is_active(
            Utc.with_ymd_and_hms(2026, 5, 18, 1, 0, 0)
                .single()
                .expect("valid UTC timestamp"),
            chrono_tz::Asia::Tokyo,
            (9, 0),
            (15, 0),
        ));
        assert!(!market_is_active(
            Utc.with_ymd_and_hms(2026, 5, 18, 7, 0, 0)
                .single()
                .expect("valid UTC timestamp"),
            chrono_tz::Asia::Tokyo,
            (9, 0),
            (15, 0),
        ));
    }

    #[test]
    fn market_clock_text_shows_close_countdown_while_active() {
        let label = market_clock_text(
            "New York",
            Utc.with_ymd_and_hms(2026, 5, 18, 14, 0, 0)
                .single()
                .expect("valid UTC timestamp"),
            chrono_tz::America::New_York,
            (9, 30),
            (16, 0),
        );

        assert_eq!(label, "New York 10:00:00 EDT (closes in 6h 0m)");
    }

    #[test]
    fn market_clock_text_shows_open_countdown_while_inactive() {
        let label = market_clock_text(
            "New York",
            Utc.with_ymd_and_hms(2026, 5, 18, 13, 0, 0)
                .single()
                .expect("valid UTC timestamp"),
            chrono_tz::America::New_York,
            (9, 30),
            (16, 0),
        );

        assert_eq!(label, "New York 09:00:00 EDT (opens in 0h 30m)");
    }
}
